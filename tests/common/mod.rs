//! Harness for the integration tests.
//!
//! The tests run against a real MySQL server. They are skipped unless
//! `TEST_DATABASE_URL` points at a throw-away database, because every test
//! truncates the `customers` table.

use std::sync::Once;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use serde_json::Value;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use tokio::sync::{Mutex, MutexGuard};
use tower::ServiceExt;

use rust_customer_rest_api_mysql::repository::MySqlCustomerRepository;
use rust_customer_rest_api_mysql::{AppState, db, routes};

/// The tests share one database, so they take turns.
static TURN: Mutex<()> = Mutex::const_new(());

static SKIP_NOTICE: Once = Once::new();

/// A router wired to the real MySQL repository, plus the pool behind it.
pub struct TestApp {
    router: Router,
    pub pool: MySqlPool,
    _turn: MutexGuard<'static, ()>,
}

/// Prepare an app on an empty `customers` table, or `None` when no test
/// database is configured.
pub async fn start() -> Option<TestApp> {
    let _ = dotenvy::dotenv();

    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        SKIP_NOTICE.call_once(|| {
            eprintln!(
                "skipping the MySQL integration tests: set TEST_DATABASE_URL to run them \
                 (see .env.example)"
            );
        });
        return None;
    };

    let turn = TURN.lock().await;

    // Each test gets its own pool: a `sqlx` pool belongs to the runtime that
    // created it, and `#[tokio::test]` gives every test a runtime of its own.
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&url)
        .await
        .expect("failed to connect to TEST_DATABASE_URL");

    db::run_migrations(&pool)
        .await
        .expect("failed to migrate the test database");

    sqlx::query("TRUNCATE TABLE customers")
        .execute(&pool)
        .await
        .expect("failed to empty the customers table");

    let router = routes::router(AppState::new(MySqlCustomerRepository::new(pool.clone())));

    Some(TestApp {
        router,
        pool,
        _turn: turn,
    })
}

impl TestApp {
    pub async fn request(
        &self,
        method: Method,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        let request = match body {
            Some(json) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                builder.body(Body::from(json.to_string())).unwrap()
            }
            None => builder.body(Body::empty()).unwrap(),
        };

        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };

        (status, json)
    }

    pub async fn get(&self, uri: &str) -> (StatusCode, Value) {
        self.request(Method::GET, uri, None).await
    }

    pub async fn post(&self, uri: &str, body: Value) -> (StatusCode, Value) {
        self.request(Method::POST, uri, Some(body)).await
    }

    pub async fn put(&self, uri: &str, body: Value) -> (StatusCode, Value) {
        self.request(Method::PUT, uri, Some(body)).await
    }

    pub async fn delete(&self, uri: &str) -> (StatusCode, Value) {
        self.request(Method::DELETE, uri, None).await
    }

    /// Create a customer and return its id.
    pub async fn seed(&self, name: &str, email: &str) -> u64 {
        let (status, body) = self
            .post(
                "/api/v1/customers",
                serde_json::json!({ "name": name, "email": email }),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "seeding failed: {body}");

        body["id"].as_u64().expect("the response carries an id")
    }

    /// How many rows the table actually holds.
    pub async fn row_count(&self) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM customers")
            .fetch_one(&self.pool)
            .await
            .expect("failed to count the customers")
    }

    /// The email stored for `id`, straight from MySQL.
    pub async fn stored_email(&self, id: u64) -> Option<String> {
        sqlx::query_scalar("SELECT email FROM customers WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .expect("failed to read the customer")
    }
}
