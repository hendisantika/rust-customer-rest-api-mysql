use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde_json::json;
use validator::Validate;

use crate::AppState;
use crate::error::{AppError, ErrorResponse};
use crate::models::{CreateCustomer, Customer, CustomerPage, ListCustomersQuery, UpdateCustomer};
use crate::repository;

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
pub async fn create_customer(
    State(state): State<AppState>,
    Json(payload): Json<CreateCustomer>,
) -> Result<(StatusCode, Json<Customer>), AppError> {
    payload.validate()?;
    let customer = repository::create(&state.pool, &payload).await?;
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
pub async fn list_customers(
    State(state): State<AppState>,
    Query(query): Query<ListCustomersQuery>,
) -> Result<Json<CustomerPage>, AppError> {
    query.validate()?;
    let page = repository::list(&state.pool, &query).await?;
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
pub async fn get_customer(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Customer>, AppError> {
    let customer = repository::find_by_id(&state.pool, id).await?;
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
pub async fn update_customer(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(payload): Json<UpdateCustomer>,
) -> Result<Json<Customer>, AppError> {
    payload.validate()?;
    let customer = repository::update(&state.pool, id, &payload).await?;
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
pub async fn delete_customer(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<StatusCode, AppError> {
    repository::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
