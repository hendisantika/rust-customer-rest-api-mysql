//! Customer REST API: axum handlers over a MySQL-backed repository, described
//! with OpenAPI 3.

pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod models;
pub mod openapi;
pub mod repository;
pub mod routes;
#[cfg(test)]
mod test_support;

use std::sync::Arc;

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
