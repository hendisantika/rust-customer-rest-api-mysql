mod config;
mod db;
mod error;
mod handlers;
mod models;
mod repository;
mod routes;

use std::error::Error;

use sqlx::mysql::MySqlPool;
use tokio::net::TcpListener;
use tokio::signal;
use tracing_subscriber::EnvFilter;

use crate::config::Config;

/// Shared state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: MySqlPool,
}

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

    let app = routes::router(AppState { pool });

    let listener = TcpListener::bind(config.server_addr).await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);

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
