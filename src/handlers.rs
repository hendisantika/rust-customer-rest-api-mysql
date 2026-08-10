use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde_json::json;
use validator::Validate;

use crate::AppState;
use crate::error::AppError;
use crate::models::{CreateCustomer, Customer, CustomerPage, ListCustomersQuery, UpdateCustomer};
use crate::repository;

/// Liveness probe.
pub async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

/// Create a customer.
pub async fn create_customer(
    State(state): State<AppState>,
    Json(payload): Json<CreateCustomer>,
) -> Result<(StatusCode, Json<Customer>), AppError> {
    payload.validate()?;
    let customer = repository::create(&state.pool, &payload).await?;
    Ok((StatusCode::CREATED, Json(customer)))
}

/// List customers, paginated and optionally filtered by `q`.
pub async fn list_customers(
    State(state): State<AppState>,
    Query(query): Query<ListCustomersQuery>,
) -> Result<Json<CustomerPage>, AppError> {
    query.validate()?;
    let page = repository::list(&state.pool, &query).await?;
    Ok(Json(page))
}

/// Fetch a single customer by id.
pub async fn get_customer(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Customer>, AppError> {
    let customer = repository::find_by_id(&state.pool, id).await?;
    Ok(Json(customer))
}

/// Replace a customer.
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
pub async fn delete_customer(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<StatusCode, AppError> {
    repository::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
