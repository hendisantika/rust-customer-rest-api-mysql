# rust-customer-rest-api-mysql

Customer CRUD REST API written in Rust with [axum](https://github.com/tokio-rs/axum)
and [sqlx](https://github.com/launchbadge/sqlx), backed by **MySQL 26.7.0** and
documented with **OpenAPI 3** through Swagger UI.

## Stack

| Concern       | Choice                              |
|---------------|-------------------------------------|
| HTTP          | axum 0.8 + tower-http (trace, CORS) |
| Database      | MySQL 26.7.0 via sqlx 0.9           |
| Migrations    | sqlx, embedded from `./migrations`  |
| Validation    | validator 0.21                      |
| Documentation | utoipa 5 + Swagger UI               |

## Getting started

```bash
cp .env.example .env       # adjust credentials if needed
docker compose up -d       # starts MySQL 26.7.0
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

## Docker image

Every push to `main` that passes CI publishes an image to Docker Hub, tagged
with the GitHub Actions run number and with `latest`:

```bash
docker run --rm -p 8080:8080 \
  -e DATABASE_URL='mysql://customer:secret@host.docker.internal:3306/customer_db' \
  hendisantika/rust-customer-rest-api-mysql:latest
```

The publish job needs two repository secrets, and skips itself with a notice
until both are set:

| Secret                | Value                                        |
|-----------------------|----------------------------------------------|
| `DOCKERHUB_USERNAME`  | Your Docker Hub account name                  |
| `DOCKERHUB_TOKEN`     | A Docker Hub access token with write access   |

To build it locally:

```bash
docker build -t rust-customer-rest-api-mysql:local .
```

## Deployment

Every push to `main` that passes CI and publishes an image also rolls it out
to the **dev** environment over SSH: the host pulls the freshly tagged image,
replaces the `customer-api-dev` container and waits for `/health` to answer
before the job is allowed to succeed. The app applies its own migrations on
start-up, so no separate migration step is needed.

The job is driven entirely by repository secrets, and skips itself with a
notice while `SSH_PRIVATE_KEY` is unset:

| Secret           | Purpose                                       |
|------------------|-----------------------------------------------|
| `SSH_HOST`       | Dev host                                      |
| `SSH_PORT`       | SSH port                                      |
| `SSH_USERNAME`   | SSH user, must be able to run `docker`        |
| `SSH_PRIVATE_KEY`| Private key for that user (no passphrase)     |
| `DB_HOST`        | MySQL host reachable from the dev container   |
| `DB_PORT`        | MySQL port                                    |
| `DB_NAME`        | Database name                                 |
| `DB_USERNAME`    | Database user                                 |
| `DB_PASSWORD`    | Database password                             |

| Variable        | Default  | Purpose                                  |
|-----------------|----------|------------------------------------------|
| `DEV_APP_URL`   | —        | Shown as the environment URL in GitHub   |
| `DEV_APP_PORT`  | `8080`   | Host port published by the container     |

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
├── lib.rs          # module tree and AppState
├── error.rs        # error type and its HTTP representation
└── test_support.rs # in-memory repository used by the tests
migrations/         # sqlx migrations, applied on start-up
tests/              # integration tests against a real MySQL
Dockerfile          # multi-stage build of the release image
.github/workflows/  # CI: format, clippy, tests, Docker Hub publish
```

## Tests

```bash
cargo test                      # unit tests only, no database needed
docker compose up -d            # then the integration tests too
cargo test
```

**Unit tests** (`src/handlers.rs`) drive the real router with an in-memory
`CustomerRepository`, so they need no database and no running server. They
cover the status codes and payloads of every endpoint, including validation
failures, duplicate emails, missing customers and the generic 500 returned
when the repository fails.

**Integration tests** (`tests/customer_api.rs`) run the same router over a
real MySQL connection and cover what an in-memory double cannot: the unique
index behind the 409, LIKE wildcard escaping, the case-insensitive collation,
utf8mb4 round trips, MySQL-generated timestamps and migration idempotence.

They truncate the `customers` table, so they only run when
`TEST_DATABASE_URL` points at a throw-away database and are skipped
otherwise. `docker compose up -d` creates that database (`customer_db_test`)
on first start, and `.env.example` already points at it.

```bash
TEST_DATABASE_URL=mysql://customer:secret@127.0.0.1:3306/customer_db_test \
  cargo test --test customer_api
```
