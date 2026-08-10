use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// A customer row as stored in MySQL.
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct Customer {
    #[schema(example = 1)]
    pub id: u64,
    #[schema(example = "Hendi Santika")]
    pub name: String,
    #[schema(example = "hendisantika@yahoo.co.id")]
    pub email: String,
    #[schema(example = "+6281234567890")]
    pub phone: Option<String>,
    #[schema(example = "Bandung, Indonesia")]
    pub address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Body of `POST /api/v1/customers`.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateCustomer {
    #[validate(length(min = 1, max = 120, message = "must be between 1 and 120 characters"))]
    #[schema(example = "Hendi Santika")]
    pub name: String,

    #[validate(email(message = "must be a valid email address"))]
    #[validate(length(max = 180, message = "must be at most 180 characters"))]
    #[schema(example = "hendisantika@yahoo.co.id")]
    pub email: String,

    #[validate(length(max = 30, message = "must be at most 30 characters"))]
    #[schema(example = "+6281234567890")]
    pub phone: Option<String>,

    #[validate(length(max = 255, message = "must be at most 255 characters"))]
    #[schema(example = "Bandung, Indonesia")]
    pub address: Option<String>,
}

/// Body of `PUT /api/v1/customers/{id}`; replaces the whole customer.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpdateCustomer {
    #[validate(length(min = 1, max = 120, message = "must be between 1 and 120 characters"))]
    #[schema(example = "Hendi Santika")]
    pub name: String,

    #[validate(email(message = "must be a valid email address"))]
    #[validate(length(max = 180, message = "must be at most 180 characters"))]
    #[schema(example = "hendisantika@yahoo.co.id")]
    pub email: String,

    #[validate(length(max = 30, message = "must be at most 30 characters"))]
    #[schema(example = "+6281234567890")]
    pub phone: Option<String>,

    #[validate(length(max = 255, message = "must be at most 255 characters"))]
    #[schema(example = "Bandung, Indonesia")]
    pub address: Option<String>,
}

/// Query string of `GET /api/v1/customers`.
#[derive(Debug, Clone, Deserialize, Validate, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListCustomersQuery {
    /// 1-based page number.
    #[serde(default = "default_page")]
    #[validate(range(min = 1, message = "must be at least 1"))]
    #[param(example = 1, minimum = 1)]
    pub page: u32,

    /// Number of customers per page.
    #[serde(default = "default_per_page")]
    #[validate(range(min = 1, max = 100, message = "must be between 1 and 100"))]
    #[param(example = 20, minimum = 1, maximum = 100)]
    pub per_page: u32,

    /// Free text filter matched against name and email.
    #[param(example = "hendi")]
    pub q: Option<String>,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

impl ListCustomersQuery {
    pub fn offset(&self) -> u64 {
        u64::from(self.page.saturating_sub(1)) * u64::from(self.per_page)
    }
}

/// One page of customers plus the pagination metadata.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CustomerPage {
    pub data: Vec<Customer>,
    pub page: u32,
    pub per_page: u32,
    pub total: i64,
    pub total_pages: u32,
}
