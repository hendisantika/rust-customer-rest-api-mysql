//! Integration tests against a real MySQL server.
//!
//! Run them with:
//!
//! ```bash
//! docker compose up -d
//! TEST_DATABASE_URL=mysql://customer:secret@127.0.0.1:3306/customer_db_test cargo test --test customer_api
//! ```
//!
//! Without `TEST_DATABASE_URL` every test returns early, so `cargo test` still
//! passes on a machine with no database.

mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};

#[tokio::test]
async fn create_persists_the_customer_in_mysql() {
    let Some(app) = common::start().await else {
        return;
    };

    let (status, created) = app
        .post(
            "/api/v1/customers",
            json!({
                "name": "Hendi Santika",
                "email": "hendisantika@yahoo.co.id",
                "phone": "+6281234567890",
                "address": "Bandung",
            }),
        )
        .await;

    assert_eq!(status, StatusCode::CREATED);
    let id = created["id"].as_u64().unwrap();
    assert_eq!(app.row_count().await, 1);
    assert_eq!(
        app.stored_email(id).await.as_deref(),
        Some("hendisantika@yahoo.co.id")
    );

    // The row survives a round trip through a fresh SELECT.
    let (status, fetched) = app.get(&format!("/api/v1/customers/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched, created);
}

#[tokio::test]
async fn the_unique_index_turns_a_duplicate_email_into_409() {
    let Some(app) = common::start().await else {
        return;
    };

    app.seed("Budi", "budi@example.com").await;

    let (status, body) = app
        .post(
            "/api/v1/customers",
            json!({ "name": "Someone Else", "email": "budi@example.com" }),
        )
        .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("budi@example.com")
    );
    assert_eq!(app.row_count().await, 1, "the insert must not have landed");
}

#[tokio::test]
async fn updating_to_an_email_owned_by_someone_else_conflicts() {
    let Some(app) = common::start().await else {
        return;
    };

    app.seed("Budi", "budi@example.com").await;
    let ani = app.seed("Ani", "ani@example.com").await;

    let (status, _) = app
        .put(
            &format!("/api/v1/customers/{ani}"),
            json!({ "name": "Ani", "email": "budi@example.com" }),
        )
        .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        app.stored_email(ani).await.as_deref(),
        Some("ani@example.com"),
        "the row must be left untouched"
    );
}

#[tokio::test]
async fn updating_with_unchanged_values_still_returns_the_customer() {
    let Some(app) = common::start().await else {
        return;
    };

    let id = app.seed("Budi", "budi@example.com").await;

    // MySQL reports zero affected rows here, which must not read as "missing".
    let (status, body) = app
        .put(
            &format!("/api/v1/customers/{id}"),
            json!({ "name": "Budi", "email": "budi@example.com" }),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], id);
    assert_eq!(body["name"], "Budi");
}

#[tokio::test]
async fn update_writes_every_column_including_nulls() {
    let Some(app) = common::start().await else {
        return;
    };

    let id = app
        .post(
            "/api/v1/customers",
            json!({ "name": "Budi", "email": "budi@example.com", "phone": "+62811", "address": "Bandung" }),
        )
        .await
        .1["id"]
        .as_u64()
        .unwrap();

    let (status, body) = app
        .put(
            &format!("/api/v1/customers/{id}"),
            json!({
                "name": "Budi Santoso",
                "email": "budi.santoso@example.com",
                "phone": Value::Null,
                "address": "Jakarta",
            }),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Budi Santoso");
    assert_eq!(body["phone"], Value::Null);
    assert_eq!(body["address"], "Jakarta");
    assert_eq!(
        app.stored_email(id).await.as_deref(),
        Some("budi.santoso@example.com")
    );
}

#[tokio::test]
async fn delete_removes_the_row() {
    let Some(app) = common::start().await else {
        return;
    };

    let id = app.seed("Budi", "budi@example.com").await;

    let (status, _) = app.delete(&format!("/api/v1/customers/{id}")).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(app.row_count().await, 0);

    let (status, _) = app.delete(&format!("/api/v1/customers/{id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_pages_through_the_table_newest_first() {
    let Some(app) = common::start().await else {
        return;
    };

    app.seed("First", "first@example.com").await;
    app.seed("Second", "second@example.com").await;
    app.seed("Third", "third@example.com").await;

    let (status, page1) = app.get("/api/v1/customers?page=1&per_page=2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page1["total"], 3);
    assert_eq!(page1["total_pages"], 2);
    assert_eq!(page1["data"][0]["email"], "third@example.com");
    assert_eq!(page1["data"][1]["email"], "second@example.com");

    let (_, page2) = app.get("/api/v1/customers?page=2&per_page=2").await;
    assert_eq!(page2["data"].as_array().unwrap().len(), 1);
    assert_eq!(page2["data"][0]["email"], "first@example.com");

    let (_, page3) = app.get("/api/v1/customers?page=3&per_page=2").await;
    assert!(page3["data"].as_array().unwrap().is_empty());
    assert_eq!(page3["total"], 3);
}

#[tokio::test]
async fn the_filter_matches_name_or_email_case_insensitively() {
    let Some(app) = common::start().await else {
        return;
    };

    app.seed("Hendi Santika", "hendisantika@yahoo.co.id").await;
    app.seed("Budi", "budi@example.com").await;

    // utf8mb4_0900_ai_ci is case insensitive.
    let (_, by_name) = app.get("/api/v1/customers?q=HENDI").await;
    assert_eq!(by_name["total"], 1);
    assert_eq!(by_name["data"][0]["name"], "Hendi Santika");

    let (_, by_email) = app.get("/api/v1/customers?q=example.com").await;
    assert_eq!(by_email["total"], 1);
    assert_eq!(by_email["data"][0]["name"], "Budi");
}

#[tokio::test]
async fn the_filter_treats_like_wildcards_as_literal_text() {
    let Some(app) = common::start().await else {
        return;
    };

    app.seed("100% Cotton", "cotton@example.com").await;
    app.seed("Budi", "budi@example.com").await;
    app.seed("a_b", "underscore@example.com").await;
    app.seed("axb", "axb@example.com").await;

    // `%` would match every row if it reached MySQL unescaped.
    let (_, percent) = app.get("/api/v1/customers?q=%25").await;
    assert_eq!(percent["total"], 1);
    assert_eq!(percent["data"][0]["name"], "100% Cotton");

    // `_` would match any single character.
    let (_, underscore) = app.get("/api/v1/customers?q=a_b").await;
    assert_eq!(underscore["total"], 1);
    assert_eq!(underscore["data"][0]["name"], "a_b");
}

#[tokio::test]
async fn a_blank_filter_lists_everything() {
    let Some(app) = common::start().await else {
        return;
    };

    app.seed("Budi", "budi@example.com").await;
    app.seed("Ani", "ani@example.com").await;

    let (status, body) = app.get("/api/v1/customers?q=%20%20").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 2);
}

#[tokio::test]
async fn mysql_fills_in_the_timestamps() {
    let Some(app) = common::start().await else {
        return;
    };

    let id = app.seed("Budi", "budi@example.com").await;
    let (_, created) = app.get(&format!("/api/v1/customers/{id}")).await;

    let created_at = created["created_at"].as_str().unwrap().to_owned();
    assert!(
        created_at.ends_with('Z'),
        "timestamps are serialised as UTC"
    );
    assert_eq!(created["updated_at"].as_str().unwrap(), created_at);

    let (_, updated) = app
        .put(
            &format!("/api/v1/customers/{id}"),
            json!({ "name": "Budi Santoso", "email": "budi@example.com" }),
        )
        .await;

    assert_eq!(
        updated["created_at"].as_str().unwrap(),
        created_at,
        "created_at must not move"
    );
    assert!(updated["updated_at"].as_str().unwrap() >= created_at.as_str());
}

#[tokio::test]
async fn utf8mb4_text_survives_the_round_trip() {
    let Some(app) = common::start().await else {
        return;
    };

    let (status, created) = app
        .post(
            "/api/v1/customers",
            json!({
                "name": "Hendi 🦀 Santika",
                "email": "unicode@example.com",
                "address": "Jl. Merdeka №1, Bandung — Jawa Barat",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);

    let id = created["id"].as_u64().unwrap();
    let (_, fetched) = app.get(&format!("/api/v1/customers/{id}")).await;

    assert_eq!(fetched["name"], "Hendi 🦀 Santika");
    assert_eq!(fetched["address"], "Jl. Merdeka №1, Bandung — Jawa Barat");
}

#[tokio::test]
async fn a_name_at_the_column_limit_is_accepted() {
    let Some(app) = common::start().await else {
        return;
    };

    let name = "x".repeat(120);
    let (status, body) = app
        .post(
            "/api/v1/customers",
            json!({ "name": name, "email": "limit@example.com" }),
        )
        .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], name);

    // One character more is rejected before it reaches MySQL.
    let (status, _) = app
        .post(
            "/api/v1/customers",
            json!({ "name": "x".repeat(121), "email": "over@example.com" }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn migrations_are_idempotent() {
    let Some(app) = common::start().await else {
        return;
    };

    // The harness already migrated; running again must be a no-op.
    rust_customer_rest_api_mysql::db::run_migrations(&app.pool)
        .await
        .expect("re-running the migrations must succeed");

    let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(applied, 1);
}
