use std::future::Future;

use sqlx::mysql::MySqlPool;

use crate::error::AppError;
use crate::models::{CreateCustomer, Customer, CustomerPage, ListCustomersQuery, UpdateCustomer};

/// Storage behind the customer endpoints.
///
/// The handlers depend on this trait rather than on MySQL directly, so they can
/// be exercised in tests without a database.
pub trait CustomerRepository: Send + Sync + 'static {
    fn create(
        &self,
        input: &CreateCustomer,
    ) -> impl Future<Output = Result<Customer, AppError>> + Send;

    fn find_by_id(&self, id: u64) -> impl Future<Output = Result<Customer, AppError>> + Send;

    fn list(
        &self,
        query: &ListCustomersQuery,
    ) -> impl Future<Output = Result<CustomerPage, AppError>> + Send;

    fn update(
        &self,
        id: u64,
        input: &UpdateCustomer,
    ) -> impl Future<Output = Result<Customer, AppError>> + Send;

    fn delete(&self, id: u64) -> impl Future<Output = Result<(), AppError>> + Send;
}

/// The MySQL implementation of [`CustomerRepository`].
#[derive(Debug, Clone)]
pub struct MySqlCustomerRepository {
    pool: MySqlPool,
}

impl MySqlCustomerRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

impl CustomerRepository for MySqlCustomerRepository {
    async fn create(&self, input: &CreateCustomer) -> Result<Customer, AppError> {
        let result =
            sqlx::query("INSERT INTO customers (name, email, phone, address) VALUES (?, ?, ?, ?)")
                .bind(&input.name)
                .bind(&input.email)
                .bind(&input.phone)
                .bind(&input.address)
                .execute(&self.pool)
                .await
                .map_err(|error| duplicate_email(error, &input.email))?;

        self.find_by_id(result.last_insert_id()).await
    }

    async fn find_by_id(&self, id: u64) -> Result<Customer, AppError> {
        sqlx::query_as::<_, Customer>(
            "SELECT id, name, email, phone, address, created_at, updated_at \
             FROM customers WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::NotFound(id))
    }

    async fn list(&self, query: &ListCustomersQuery) -> Result<CustomerPage, AppError> {
        let pattern = query
            .q
            .as_deref()
            .map(str::trim)
            .filter(|term| !term.is_empty())
            .map(|term| format!("%{}%", escape_like(term)));

        // `? IS NULL` keeps the filter optional without building the SQL by hand.
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM customers \
             WHERE (? IS NULL OR name LIKE ? ESCAPE '\\\\' OR email LIKE ? ESCAPE '\\\\')",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(&self.pool)
        .await?;

        let data = sqlx::query_as::<_, Customer>(
            "SELECT id, name, email, phone, address, created_at, updated_at \
             FROM customers \
             WHERE (? IS NULL OR name LIKE ? ESCAPE '\\\\' OR email LIKE ? ESCAPE '\\\\') \
             ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(query.per_page)
        .bind(query.offset())
        .fetch_all(&self.pool)
        .await?;

        let total_pages = total.unsigned_abs().div_ceil(u64::from(query.per_page)) as u32;

        Ok(CustomerPage {
            data,
            page: query.page,
            per_page: query.per_page,
            total,
            total_pages,
        })
    }

    async fn update(&self, id: u64, input: &UpdateCustomer) -> Result<Customer, AppError> {
        // MySQL reports zero affected rows when the update is a no-op, so the row
        // is looked up first to tell "missing" apart from "unchanged".
        self.find_by_id(id).await?;

        sqlx::query(
            "UPDATE customers SET name = ?, email = ?, phone = ?, address = ? WHERE id = ?",
        )
        .bind(&input.name)
        .bind(&input.email)
        .bind(&input.phone)
        .bind(&input.address)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|error| duplicate_email(error, &input.email))?;

        self.find_by_id(id).await
    }

    async fn delete(&self, id: u64) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM customers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(id));
        }

        Ok(())
    }
}

/// Translate a unique index violation on `uk_customers_email` into a 409.
fn duplicate_email(error: sqlx::Error, email: &str) -> AppError {
    match &error {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            AppError::DuplicateEmail(email.to_owned())
        }
        _ => AppError::Database(error),
    }
}

/// Neutralise the LIKE wildcards a caller may have typed into `q`.
fn escape_like(term: &str) -> String {
    term.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
