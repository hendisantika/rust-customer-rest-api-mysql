use utoipa::OpenApi;

use crate::error::ErrorResponse;
use crate::handlers;
use crate::models::{CreateCustomer, Customer, CustomerPage, UpdateCustomer};

/// OpenAPI 3 description of the whole HTTP surface.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Customer REST API",
        version = "0.1.0",
        description = "CRUD REST API for customers, backed by MySQL 9.7.0.",
        license(name = "MIT")
    ),
    servers((url = "/", description = "This server")),
    paths(
        handlers::health,
        handlers::list_customers,
        handlers::create_customer,
        handlers::get_customer,
        handlers::update_customer,
        handlers::delete_customer,
    ),
    components(schemas(
        Customer,
        CreateCustomer,
        UpdateCustomer,
        CustomerPage,
        ErrorResponse,
    )),
    tags(
        (name = "customers", description = "Create, read, update and delete customers"),
        (name = "health", description = "Service health"),
    )
)]
pub struct ApiDoc;
