use axum::Router;
use axum::routing::get;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::AppState;
use crate::handlers;
use crate::openapi::ApiDoc;

/// Where the generated OpenAPI 3 document is served from.
pub const OPENAPI_PATH: &str = "/api-docs/openapi.json";

/// Where Swagger UI is mounted.
pub const SWAGGER_UI_PATH: &str = "/swagger-ui";

pub fn router(state: AppState) -> Router {
    let customers = Router::new()
        .route(
            "/customers",
            get(handlers::list_customers).post(handlers::create_customer),
        )
        .route(
            "/customers/{id}",
            get(handlers::get_customer)
                .put(handlers::update_customer)
                .delete(handlers::delete_customer),
        );

    let api = Router::new()
        .route("/health", get(handlers::health))
        .nest("/api/v1", customers)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    api.merge(SwaggerUi::new(SWAGGER_UI_PATH).url(OPENAPI_PATH, ApiDoc::openapi()))
}
