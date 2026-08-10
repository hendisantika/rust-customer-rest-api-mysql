use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde_json::json;
use validator::Validate;

use crate::AppState;
use crate::error::{AppError, ErrorResponse};
use crate::models::{CreateCustomer, Customer, CustomerPage, ListCustomersQuery, UpdateCustomer};
use crate::repository::CustomerRepository;

/// Liveness probe.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "The service is up", body = serde_json::Value,
         example = json!({ "status": "ok" }))
    )
)]
pub async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

/// Create a customer.
#[utoipa::path(
    post,
    path = "/api/v1/customers",
    tag = "customers",
    request_body = CreateCustomer,
    responses(
        (status = 201, description = "Customer created", body = Customer),
        (status = 409, description = "Email already taken", body = ErrorResponse),
        (status = 422, description = "Validation failed", body = ErrorResponse),
        (status = 500, description = "Unexpected error", body = ErrorResponse)
    )
)]
pub async fn create_customer<R: CustomerRepository>(
    State(state): State<AppState<R>>,
    Json(payload): Json<CreateCustomer>,
) -> Result<(StatusCode, Json<Customer>), AppError> {
    payload.validate()?;
    let customer = state.repo.create(&payload).await?;
    Ok((StatusCode::CREATED, Json(customer)))
}

/// List customers, paginated and optionally filtered by `q`.
#[utoipa::path(
    get,
    path = "/api/v1/customers",
    tag = "customers",
    params(ListCustomersQuery),
    responses(
        (status = 200, description = "A page of customers", body = CustomerPage),
        (status = 422, description = "Invalid pagination parameters", body = ErrorResponse),
        (status = 500, description = "Unexpected error", body = ErrorResponse)
    )
)]
pub async fn list_customers<R: CustomerRepository>(
    State(state): State<AppState<R>>,
    Query(query): Query<ListCustomersQuery>,
) -> Result<Json<CustomerPage>, AppError> {
    query.validate()?;
    let page = state.repo.list(&query).await?;
    Ok(Json(page))
}

/// Fetch a single customer by id.
#[utoipa::path(
    get,
    path = "/api/v1/customers/{id}",
    tag = "customers",
    params(("id" = u64, Path, description = "Customer identifier", example = 1)),
    responses(
        (status = 200, description = "The customer", body = Customer),
        (status = 404, description = "Customer not found", body = ErrorResponse),
        (status = 500, description = "Unexpected error", body = ErrorResponse)
    )
)]
pub async fn get_customer<R: CustomerRepository>(
    State(state): State<AppState<R>>,
    Path(id): Path<u64>,
) -> Result<Json<Customer>, AppError> {
    let customer = state.repo.find_by_id(id).await?;
    Ok(Json(customer))
}

/// Replace a customer.
#[utoipa::path(
    put,
    path = "/api/v1/customers/{id}",
    tag = "customers",
    params(("id" = u64, Path, description = "Customer identifier", example = 1)),
    request_body = UpdateCustomer,
    responses(
        (status = 200, description = "The updated customer", body = Customer),
        (status = 404, description = "Customer not found", body = ErrorResponse),
        (status = 409, description = "Email already taken", body = ErrorResponse),
        (status = 422, description = "Validation failed", body = ErrorResponse),
        (status = 500, description = "Unexpected error", body = ErrorResponse)
    )
)]
pub async fn update_customer<R: CustomerRepository>(
    State(state): State<AppState<R>>,
    Path(id): Path<u64>,
    Json(payload): Json<UpdateCustomer>,
) -> Result<Json<Customer>, AppError> {
    payload.validate()?;
    let customer = state.repo.update(id, &payload).await?;
    Ok(Json(customer))
}

/// Delete a customer.
#[utoipa::path(
    delete,
    path = "/api/v1/customers/{id}",
    tag = "customers",
    params(("id" = u64, Path, description = "Customer identifier", example = 1)),
    responses(
        (status = 204, description = "Customer deleted"),
        (status = 404, description = "Customer not found", body = ErrorResponse),
        (status = 500, description = "Unexpected error", body = ErrorResponse)
    )
)]
pub async fn delete_customer<R: CustomerRepository>(
    State(state): State<AppState<R>>,
    Path(id): Path<u64>,
) -> Result<StatusCode, AppError> {
    state.repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode, header};
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::AppState;
    use crate::repository::CustomerRepository;
    use crate::routes;
    use crate::test_support::InMemoryCustomerRepository;

    fn app<R: CustomerRepository>(repo: R) -> Router {
        routes::router(AppState::new(repo))
    }

    /// Drive one request through the real router and decode the response.
    async fn send(
        app: Router,
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

        let response = app.oneshot(request).await.unwrap();
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

    async fn get(app: Router, uri: &str) -> (StatusCode, Value) {
        send(app, Method::GET, uri, None).await
    }

    fn payload(name: &str, email: &str) -> Value {
        serde_json::json!({ "name": name, "email": email })
    }

    #[tokio::test]
    async fn health_reports_ok() {
        let (status, body) = get(app(InMemoryCustomerRepository::new()), "/health").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    async fn create_customer_returns_201_with_the_stored_customer() {
        let (status, body) = send(
            app(InMemoryCustomerRepository::new()),
            Method::POST,
            "/api/v1/customers",
            Some(serde_json::json!({
                "name": "Hendi Santika",
                "email": "hendisantika@yahoo.co.id",
                "phone": "+6281234567890",
                "address": "Bandung",
            })),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["id"], 1);
        assert_eq!(body["name"], "Hendi Santika");
        assert_eq!(body["email"], "hendisantika@yahoo.co.id");
        assert_eq!(body["phone"], "+6281234567890");
        assert_eq!(body["address"], "Bandung");
        assert!(body["created_at"].is_string());
        assert!(body["updated_at"].is_string());
    }

    #[tokio::test]
    async fn create_customer_rejects_an_invalid_payload() {
        let (status, body) = send(
            app(InMemoryCustomerRepository::new()),
            Method::POST,
            "/api/v1/customers",
            Some(payload("", "not-an-email")),
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["status"], 422);
        assert!(body["details"]["name"].is_array());
        assert_eq!(body["details"]["email"][0]["code"], "email");
    }

    #[tokio::test]
    async fn create_customer_rejects_a_duplicate_email() {
        let repo = InMemoryCustomerRepository::new().with_customer("Budi", "budi@example.com");

        let (status, body) = send(
            app(repo),
            Method::POST,
            "/api/v1/customers",
            Some(payload("Someone Else", "budi@example.com")),
        )
        .await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"], "Conflict");
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .contains("budi@example.com")
        );
    }

    #[tokio::test]
    async fn create_customer_rejects_malformed_json() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/customers")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{ not json"))
            .unwrap();

        let response = app(InMemoryCustomerRepository::new())
            .oneshot(request)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_customers_uses_the_default_pagination() {
        let repo = InMemoryCustomerRepository::new()
            .with_customer("Budi", "budi@example.com")
            .with_customer("Ani", "ani@example.com");

        let (status, body) = get(app(repo), "/api/v1/customers").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["page"], 1);
        assert_eq!(body["per_page"], 20);
        assert_eq!(body["total"], 2);
        assert_eq!(body["total_pages"], 1);
        // Newest first.
        assert_eq!(body["data"][0]["email"], "ani@example.com");
        assert_eq!(body["data"][1]["email"], "budi@example.com");
    }

    #[tokio::test]
    async fn list_customers_honours_page_and_per_page() {
        let repo = InMemoryCustomerRepository::new()
            .with_customer("Budi", "budi@example.com")
            .with_customer("Ani", "ani@example.com")
            .with_customer("Cici", "cici@example.com");

        let (status, body) = get(app(repo), "/api/v1/customers?page=2&per_page=1").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["page"], 2);
        assert_eq!(body["per_page"], 1);
        assert_eq!(body["total"], 3);
        assert_eq!(body["total_pages"], 3);
        assert_eq!(body["data"].as_array().unwrap().len(), 1);
        assert_eq!(body["data"][0]["email"], "ani@example.com");
    }

    #[tokio::test]
    async fn list_customers_filters_by_q() {
        let repo = InMemoryCustomerRepository::new()
            .with_customer("Hendi Santika", "hendisantika@yahoo.co.id")
            .with_customer("Budi", "budi@example.com");

        let (status, body) = get(app(repo), "/api/v1/customers?q=hendi").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["total"], 1);
        assert_eq!(body["data"][0]["name"], "Hendi Santika");
    }

    #[tokio::test]
    async fn list_customers_rejects_an_out_of_range_per_page() {
        let (status, body) = get(
            app(InMemoryCustomerRepository::new()),
            "/api/v1/customers?per_page=500",
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["details"]["per_page"][0]["code"], "range");
    }

    #[tokio::test]
    async fn get_customer_returns_the_customer() {
        let repo = InMemoryCustomerRepository::new().with_customer("Budi", "budi@example.com");

        let (status, body) = get(app(repo), "/api/v1/customers/1").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], 1);
        assert_eq!(body["email"], "budi@example.com");
    }

    #[tokio::test]
    async fn get_customer_returns_404_when_missing() {
        let (status, body) = get(
            app(InMemoryCustomerRepository::new()),
            "/api/v1/customers/42",
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["status"], 404);
        assert_eq!(body["message"], "customer 42 was not found");
    }

    #[tokio::test]
    async fn update_customer_replaces_every_field() {
        let repo = InMemoryCustomerRepository::new().with_customer("Budi", "budi@example.com");

        let (status, body) = send(
            app(repo),
            Method::PUT,
            "/api/v1/customers/1",
            Some(serde_json::json!({
                "name": "Budi Santoso",
                "email": "budi.santoso@example.com",
                "phone": Value::Null,
                "address": "Jakarta",
            })),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], 1);
        assert_eq!(body["name"], "Budi Santoso");
        assert_eq!(body["email"], "budi.santoso@example.com");
        assert_eq!(body["phone"], Value::Null);
        assert_eq!(body["address"], "Jakarta");
    }

    #[tokio::test]
    async fn update_customer_returns_404_when_missing() {
        let (status, _) = send(
            app(InMemoryCustomerRepository::new()),
            Method::PUT,
            "/api/v1/customers/42",
            Some(payload("Ghost", "ghost@example.com")),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_customer_rejects_an_email_taken_by_someone_else() {
        let repo = InMemoryCustomerRepository::new()
            .with_customer("Budi", "budi@example.com")
            .with_customer("Ani", "ani@example.com");

        let (status, _) = send(
            app(repo),
            Method::PUT,
            "/api/v1/customers/2",
            Some(payload("Ani", "budi@example.com")),
        )
        .await;

        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn update_customer_rejects_an_invalid_payload() {
        let repo = InMemoryCustomerRepository::new().with_customer("Budi", "budi@example.com");

        let (status, body) = send(
            app(repo),
            Method::PUT,
            "/api/v1/customers/1",
            Some(payload("Budi", "nope")),
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["details"]["email"][0]["code"], "email");
    }

    #[tokio::test]
    async fn delete_customer_returns_204_and_removes_the_row() {
        let repo = InMemoryCustomerRepository::new().with_customer("Budi", "budi@example.com");
        let app = app(repo);

        let (status, body) = send(app.clone(), Method::DELETE, "/api/v1/customers/1", None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(body, Value::Null);

        let (status, _) = get(app, "/api/v1/customers/1").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_customer_returns_404_when_missing() {
        let (status, _) = send(
            app(InMemoryCustomerRepository::new()),
            Method::DELETE,
            "/api/v1/customers/42",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn repository_failures_become_a_generic_500() {
        let (status, body) = get(
            app(InMemoryCustomerRepository::broken()),
            "/api/v1/customers/1",
        )
        .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["message"], "an internal error occurred");
        // Database internals must never leak to the client.
        assert!(body["details"].is_null());
    }

    #[tokio::test]
    async fn openapi_document_describes_the_customer_routes() {
        let (status, body) = get(
            app(InMemoryCustomerRepository::new()),
            "/api-docs/openapi.json",
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["openapi"].as_str().unwrap().starts_with("3."));
        assert!(body["paths"]["/api/v1/customers"]["post"].is_object());
        assert!(body["paths"]["/api/v1/customers"]["get"].is_object());
        assert!(body["paths"]["/api/v1/customers/{id}"]["put"].is_object());
        assert!(body["paths"]["/api/v1/customers/{id}"]["delete"].is_object());
    }
}
