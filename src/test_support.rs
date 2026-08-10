//! In-memory doubles used by the handler tests.

use std::sync::Mutex;

use chrono::Utc;

use crate::error::AppError;
use crate::models::{CreateCustomer, Customer, CustomerPage, ListCustomersQuery, UpdateCustomer};
use crate::repository::CustomerRepository;

/// A [`CustomerRepository`] backed by a `Vec`, mirroring the observable
/// behaviour of the MySQL implementation: unique emails, "not found" for
/// unknown ids, newest first ordering and offset pagination.
#[derive(Debug)]
pub struct InMemoryCustomerRepository {
    rows: Mutex<Vec<Customer>>,
    next_id: Mutex<u64>,
    /// When set, every call fails with this error instead of touching `rows`.
    broken: bool,
}

impl InMemoryCustomerRepository {
    pub fn new() -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
            broken: false,
        }
    }

    /// A repository whose every call fails the way an unreachable database would.
    pub fn broken() -> Self {
        Self {
            broken: true,
            ..Self::new()
        }
    }

    /// Seed a customer, returning the row as stored.
    pub fn with_customer(self, name: &str, email: &str) -> Self {
        {
            let mut rows = self.rows.lock().unwrap();
            let mut next_id = self.next_id.lock().unwrap();
            let now = Utc::now();
            rows.push(Customer {
                id: *next_id,
                name: name.to_owned(),
                email: email.to_owned(),
                phone: None,
                address: None,
                created_at: now,
                updated_at: now,
            });
            *next_id += 1;
        }
        self
    }

    fn guard(&self) -> Result<(), AppError> {
        if self.broken {
            return Err(AppError::Database(sqlx::Error::PoolClosed));
        }
        Ok(())
    }
}

impl CustomerRepository for InMemoryCustomerRepository {
    async fn create(&self, input: &CreateCustomer) -> Result<Customer, AppError> {
        self.guard()?;

        let mut rows = self.rows.lock().unwrap();
        if rows.iter().any(|row| row.email == input.email) {
            return Err(AppError::DuplicateEmail(input.email.clone()));
        }

        let mut next_id = self.next_id.lock().unwrap();
        let now = Utc::now();
        let customer = Customer {
            id: *next_id,
            name: input.name.clone(),
            email: input.email.clone(),
            phone: input.phone.clone(),
            address: input.address.clone(),
            created_at: now,
            updated_at: now,
        };
        *next_id += 1;
        rows.push(customer.clone());

        Ok(customer)
    }

    async fn find_by_id(&self, id: u64) -> Result<Customer, AppError> {
        self.guard()?;

        self.rows
            .lock()
            .unwrap()
            .iter()
            .find(|row| row.id == id)
            .cloned()
            .ok_or(AppError::NotFound(id))
    }

    async fn list(&self, query: &ListCustomersQuery) -> Result<CustomerPage, AppError> {
        self.guard()?;

        let term = query
            .q
            .as_deref()
            .map(str::trim)
            .filter(|term| !term.is_empty())
            .map(str::to_lowercase);

        let rows = self.rows.lock().unwrap();
        let mut matching: Vec<Customer> = rows
            .iter()
            .filter(|row| match &term {
                Some(term) => {
                    row.name.to_lowercase().contains(term)
                        || row.email.to_lowercase().contains(term)
                }
                None => true,
            })
            .cloned()
            .collect();
        matching.sort_by(|a, b| b.id.cmp(&a.id));

        let total = matching.len() as i64;
        let data = matching
            .into_iter()
            .skip(query.offset() as usize)
            .take(query.per_page as usize)
            .collect();

        Ok(CustomerPage {
            data,
            page: query.page,
            per_page: query.per_page,
            total,
            total_pages: total.unsigned_abs().div_ceil(u64::from(query.per_page)) as u32,
        })
    }

    async fn update(&self, id: u64, input: &UpdateCustomer) -> Result<Customer, AppError> {
        self.guard()?;

        let mut rows = self.rows.lock().unwrap();
        if !rows.iter().any(|row| row.id == id) {
            return Err(AppError::NotFound(id));
        }
        if rows
            .iter()
            .any(|row| row.id != id && row.email == input.email)
        {
            return Err(AppError::DuplicateEmail(input.email.clone()));
        }

        let row = rows
            .iter_mut()
            .find(|row| row.id == id)
            .expect("existence checked above");
        row.name = input.name.clone();
        row.email = input.email.clone();
        row.phone = input.phone.clone();
        row.address = input.address.clone();
        row.updated_at = Utc::now();

        Ok(row.clone())
    }

    async fn delete(&self, id: u64) -> Result<(), AppError> {
        self.guard()?;

        let mut rows = self.rows.lock().unwrap();
        let before = rows.len();
        rows.retain(|row| row.id != id);

        if rows.len() == before {
            return Err(AppError::NotFound(id));
        }

        Ok(())
    }
}
