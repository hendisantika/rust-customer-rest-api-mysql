# syntax=docker/dockerfile:1

# ---- build ----------------------------------------------------------------
FROM rust:1.97-bookworm AS builder

WORKDIR /app

# Dependencies first, so a source-only change reuses the cached layer.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && echo '' > src/lib.rs \
    && cargo build --release --locked \
    && rm -rf src

# `sqlx::migrate!` embeds the migrations at compile time, so they are needed here.
COPY migrations ./migrations
COPY src ./src

# Touch the real sources so cargo rebuilds them over the dependency cache.
RUN touch src/main.rs src/lib.rs \
    && cargo build --release --locked --bin rust-customer-rest-api-mysql

# ---- runtime --------------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 app

COPY --from=builder /app/target/release/rust-customer-rest-api-mysql /usr/local/bin/customer-api

USER app
EXPOSE 8080

ENV SERVER_ADDR=0.0.0.0:8080 \
    RUST_LOG=info

ENTRYPOINT ["/usr/local/bin/customer-api"]
