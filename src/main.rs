mod config;
mod db;
mod error;
mod handlers;
mod models;
mod openapi;
mod repository;
mod routes;
#[cfg(test)]
mod test_support;

use std::error::Error;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::signal;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::repository::MySqlCustomerRepository;

/// Shared state handed to every handler.
#[derive(Debug)]
pub struct AppState<R> {
    pub repo: Arc<R>,
}

impl<R> AppState<R> {
    pub fn new(repo: R) -> Self {
        Self {
            repo: Arc::new(repo),
        }
    }
}

// Derived `Clone` would demand `R: Clone`, which the `Arc` already spares us.
impl<R> Clone for AppState<R> {
    fn clone(&self) -> Self {
        Self {
            repo: Arc::clone(&self.repo),
        }
    }
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
