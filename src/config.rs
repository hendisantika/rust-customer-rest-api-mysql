use std::env;
use std::net::SocketAddr;

/// Runtime configuration, read once at start-up from the environment.
#[derive(Debug, Clone)]
pub struct Config {
    pub server_addr: SocketAddr,
    pub database_url: String,
    pub max_connections: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("environment variable {0} is required")]
    Missing(&'static str),
    #[error("environment variable {name} has an invalid value: {value}")]
    Invalid { name: &'static str, value: String },
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url =
            env::var("DATABASE_URL").map_err(|_| ConfigError::Missing("DATABASE_URL"))?;

        let raw_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
        let server_addr = raw_addr.parse().map_err(|_| ConfigError::Invalid {
            name: "SERVER_ADDR",
            value: raw_addr,
        })?;

        let max_connections = match env::var("DATABASE_MAX_CONNECTIONS") {
            Ok(raw) => raw.parse().map_err(|_| ConfigError::Invalid {
                name: "DATABASE_MAX_CONNECTIONS",
                value: raw,
            })?,
            Err(_) => 10,
        };

        Ok(Self {
            server_addr,
            database_url,
            max_connections,
        })
    }
}
