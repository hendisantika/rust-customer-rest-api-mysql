use std::error::Error;

use tokio::net::TcpListener;
use tokio::signal;
use tracing_subscriber::EnvFilter;

use rust_customer_rest_api_mysql::config::Config;
use rust_customer_rest_api_mysql::repository::MySqlCustomerRepository;
use rust_customer_rest_api_mysql::{AppState, db, routes};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;

    let pool = db::connect(&config).await?;
    db::run_migrations(&pool).await?;
    tracing::info!("connected to MySQL and applied pending migrations");

    let app = routes::router(AppState::new(MySqlCustomerRepository::new(pool)));

    let listener = TcpListener::bind(config.server_addr).await?;
    let addr = listener.local_addr()?;
    tracing::info!("listening on http://{addr}");
    tracing::info!("swagger ui on http://{addr}{}", routes::SWAGGER_UI_PATH);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install the Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install the SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    tracing::info!("shutdown signal received");
}
