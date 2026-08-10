-- Runs once, the first time the data volume is initialised.
-- Gives `cargo test --test customer_api` a database of its own, so the
-- integration tests never touch development data.
CREATE DATABASE IF NOT EXISTS customer_db_test
    CHARACTER SET utf8mb4
    COLLATE utf8mb4_0900_ai_ci;

GRANT ALL PRIVILEGES ON customer_db_test.* TO 'customer'@'%';
FLUSH PRIVILEGES;
