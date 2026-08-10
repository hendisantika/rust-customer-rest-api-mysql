use axum::Router;
use axum::routing::get;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::AppState;
use crate::handlers;

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

    Router::new()
        .route("/health", get(handlers::health))
        .nest("/api/v1", customers)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
