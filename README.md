# rust-customer-rest-api-mysql

Customer CRUD REST API written in Rust with [axum](https://github.com/tokio-rs/axum)
and [sqlx](https://github.com/launchbadge/sqlx), backed by **MySQL 9.7.0** and
documented with **OpenAPI 3** through Swagger UI.

## Stack

| Concern       | Choice                              |
|---------------|-------------------------------------|
| HTTP          | axum 0.8 + tower-http (trace, CORS) |
| Database      | MySQL 9.7.0 via sqlx 0.9            |
| Migrations    | sqlx, embedded from `./migrations`  |
| Validation    | validator 0.21                      |
| Documentation | utoipa 5 + Swagger UI               |

## Getting started

```bash
cp .env.example .env       # adjust credentials if needed
docker compose up -d       # starts MySQL 9.7.0
cargo run                  # applies migrations, then serves the API
```

The API listens on `SERVER_ADDR` (default `0.0.0.0:8080`).

| URL                                              | What it is           |
|--------------------------------------------------|----------------------|
| <http://localhost:8080/swagger-ui>               | Swagger UI           |
| <http://localhost:8080/api-docs/openapi.json>    | OpenAPI 3 document   |
| <http://localhost:8080/health>                   | Liveness probe       |

## Configuration

| Variable                   | Default                                                | Purpose                              |
|----------------------------|--------------------------------------------------------|--------------------------------------|
| `DATABASE_URL`             | —  (required)                                          | MySQL connection string              |
| `DATABASE_MAX_CONNECTIONS` | `10`                                                   | Connection pool size                 |
| `SERVER_ADDR`              | `0.0.0.0:8080`                                         | Listen address                       |
| `RUST_LOG`                 | `info`                                                 | Tracing filter                       |
| `MYSQL_*`                  | see `.env.example`                                     | Credentials used by docker-compose   |

## Endpoints

| Method   | Path                     | Description                                    |
|----------|--------------------------|------------------------------------------------|
| `GET`    | `/health`                | Liveness probe                                 |
| `POST`   | `/api/v1/customers`      | Create a customer                              |
| `GET`    | `/api/v1/customers`      | List customers (`page`, `per_page`, `q`)       |
| `GET`    | `/api/v1/customers/{id}` | Fetch one customer                             |
| `PUT`    | `/api/v1/customers/{id}` | Replace a customer                             |
| `DELETE` | `/api/v1/customers/{id}` | Delete a customer                              |

### Examples

```bash
# create
curl -X POST http://localhost:8080/api/v1/customers \
  -H 'content-type: application/json' \
  -d '{"name":"Hendi Santika","email":"hendisantika@yahoo.co.id","phone":"+6281234567890","address":"Bandung"}'

# list, page 1, filtered by name or email
curl 'http://localhost:8080/api/v1/customers?page=1&per_page=20&q=hendi'

# read / replace / delete
curl http://localhost:8080/api/v1/customers/1
curl -X PUT http://localhost:8080/api/v1/customers/1 \
  -H 'content-type: application/json' \
  -d '{"name":"Hendi S.","email":"hendisantika@yahoo.co.id","phone":null,"address":"Jakarta"}'
curl -X DELETE http://localhost:8080/api/v1/customers/1
```

A list response carries its pagination metadata:

```json
{
  "data": [ { "id": 1, "name": "Hendi Santika", "email": "hendisantika@yahoo.co.id",
              "phone": "+6281234567890", "address": "Bandung",
              "created_at": "2026-08-10T00:53:23Z", "updated_at": "2026-08-10T00:53:23Z" } ],
  "page": 1, "per_page": 20, "total": 1, "total_pages": 1
}
```

## Errors

Every failure returns the same JSON shape, with `details` present only for
validation failures.

| Status | When                                             |
|--------|--------------------------------------------------|
| `404`  | The customer does not exist                      |
| `409`  | The email is already taken                       |
| `422`  | The body or the query string failed validation   |
| `500`  | Unexpected error (details are logged, not returned) |

```json
{ "status": 404, "error": "Not Found", "message": "customer 42 was not found" }
```

## Project layout

```
src/
├── main.rs         # start-up: config, pool, migrations, server, graceful shutdown
├── config.rs       # environment configuration
├── db.rs           # MySQL pool and migration runner
├── models.rs       # request, response and row types plus validation rules
├── repository.rs   # CustomerRepository trait and its MySQL implementation
├── handlers.rs     # HTTP handlers and their OpenAPI annotations
├── openapi.rs      # OpenAPI 3 document
├── routes.rs       # router, middleware and Swagger UI mount
├── error.rs        # error type and its HTTP representation
└── test_support.rs # in-memory repository used by the tests
migrations/         # sqlx migrations, applied on start-up
```

## Tests

```bash
cargo test
```

The handler tests drive the real router with an in-memory
`CustomerRepository`, so they need no database and no running server. They
cover the status codes and payloads of every endpoint, including validation
failures, duplicate emails, missing customers and the generic 500 returned
when the repository fails.
