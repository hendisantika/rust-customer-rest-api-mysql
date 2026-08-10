use std::time::Duration;

use sqlx::mysql::{MySqlPool, MySqlPoolOptions};

use crate::config::Config;

/// Open the MySQL connection pool used by the whole application.
pub async fn connect(config: &Config) -> Result<MySqlPool, sqlx::Error> {
    MySqlPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&config.database_url)
        .await
}

/// Apply every pending migration in `./migrations`.
pub async fn run_migrations(pool: &MySqlPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
