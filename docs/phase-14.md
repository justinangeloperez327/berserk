# Phase 14 — PostgreSQL driver

Phase 14 implements the shared database contracts using the synchronous `postgres` 0.19.14 client.

## Delivered

- `PostgresConnection` behind the optional `postgres` feature.
- Bound execution and queries using PostgreSQL `$1`, `$2`, … placeholders.
- Conversion for booleans, signed integers, OIDs, floating-point values, text, bytes, and SQL NULL results.
- Explicit PostgreSQL capability declaration.
- Read-write and read-only transaction creation, explicit commit/rollback, and rollback-on-drop behavior inherited from the driver.
- SQLSTATE preservation and broad classification of constraint, serialization, and connection failures.
- Opt-in live integration tests using `FRAMEWORK_POSTGRES_TEST_URL`.

## Connection security

`PostgresConnection::connect_no_tls` is deliberately explicit. It is suitable only where transport security is not required, such as a protected local test socket. Production applications should construct a TLS-enabled `postgres::Client` with a compatible TLS package and pass it to `PostgresConnection::from_client`. The framework does not silently downgrade TLS.

## Type boundary

The first adapter supports the common scalar types represented by Phase 13. PostgreSQL has no unsigned 64-bit integer type, so values above `i64::MAX` fail before execution. Untyped NULL bindings also fail because PostgreSQL parameter NULLs need type information. Decimal, date/time, UUID, JSON, arrays, enums, domains, and typed NULL bindings remain explicit future work; unsupported result types return `ErrorKind::Decode` rather than being stringified or losing precision.

`Execution::last_insert_id` remains `None` for PostgreSQL. Applications should use `INSERT ... RETURNING` and `query` when they need generated values.

## Test command

```sh
FRAMEWORK_POSTGRES_TEST_URL='host=localhost user=postgres dbname=framework_test' \
  cargo test -p framework-database --features postgres
```

Without the environment variable, live tests return without connecting. Unit and contract tests still run. The test account should point to a dedicated test database. Current live tests only query and roll back; they do not create persistent objects.

## Verification status

Source, manifests, feature wiring, module paths, and test design are reviewed. Compilation, dependency resolution, and live execution remain pending because Rust tooling is unavailable. Phase 12 measurements and the earlier runtime gate also remain pending.
