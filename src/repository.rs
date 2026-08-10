use sqlx::mysql::MySqlPool;

use crate::error::AppError;
use crate::models::{CreateCustomer, Customer, CustomerPage, ListCustomersQuery, UpdateCustomer};

pub async fn create(pool: &MySqlPool, input: &CreateCustomer) -> Result<Customer, AppError> {
    let result =
        sqlx::query("INSERT INTO customers (name, email, phone, address) VALUES (?, ?, ?, ?)")
            .bind(&input.name)
            .bind(&input.email)
            .bind(&input.phone)
            .bind(&input.address)
            .execute(pool)
            .await
            .map_err(|error| duplicate_email(error, &input.email))?;

    find_by_id(pool, result.last_insert_id()).await
}

pub async fn find_by_id(pool: &MySqlPool, id: u64) -> Result<Customer, AppError> {
    sqlx::query_as::<_, Customer>(
        "SELECT id, name, email, phone, address, created_at, updated_at \
         FROM customers WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound(id))
}

pub async fn list(pool: &MySqlPool, query: &ListCustomersQuery) -> Result<CustomerPage, AppError> {
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
    .fetch_one(pool)
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
    .fetch_all(pool)
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

pub async fn update(
    pool: &MySqlPool,
    id: u64,
    input: &UpdateCustomer,
) -> Result<Customer, AppError> {
    // MySQL reports zero affected rows when the update is a no-op, so the row
    // is looked up first to tell "missing" apart from "unchanged".
    find_by_id(pool, id).await?;

    sqlx::query("UPDATE customers SET name = ?, email = ?, phone = ?, address = ? WHERE id = ?")
        .bind(&input.name)
        .bind(&input.email)
        .bind(&input.phone)
        .bind(&input.address)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|error| duplicate_email(error, &input.email))?;

    find_by_id(pool, id).await
}

pub async fn delete(pool: &MySqlPool, id: u64) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM customers WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(id));
    }

    Ok(())
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
