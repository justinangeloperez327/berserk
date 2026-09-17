# Phase 13 — Database contracts

Phase 13 adds the `database` crate and defines stable, driver-neutral boundaries before choosing driver libraries. It does not open database connections or generate SQL.

## Delivered

- Owned and borrowed database values, ordered rows, and named columns.
- SQL statements with bindings kept separate from SQL text.
- Execution results plus connection and transaction contracts.
- Explicit driver capabilities and structured database error categories.
- Disabled-by-default `postgres`, `mysql`, and `sqlite` Cargo features in one database package.
- Contract tests for bindings, row invariants, and capabilities.

## Decisions

- Terminal connection methods execute; statement construction does not.
- Bind values never interpolate into SQL strings.
- Transactions consume themselves on commit or rollback, preventing reuse through the same handle.
- Common behavior uses shared contracts. Backend-only behavior must use explicit capabilities or driver APIs.
- Connection pooling is deferred until driver behavior and concurrency requirements can be tested.
- Date/time, decimal, UUID, and JSON conversions are deferred until concrete driver type behavior is selected; they must not be silently reduced to imprecise floating-point or ambiguous text.

## Feature use

```toml
berserk-database = { version = "0.1", features = ["postgres"] }
```

One or several driver features may be enabled. Phase 13 exposes only their compile-time boundaries. Phase 14 implements PostgreSQL; Phase 15 implements MySQL and SQLite.

The main package forwards the same names, so an application may use `framework = { features = ["postgres"] }` and access the contracts through `framework::database`.

## Verification status

Manifests, module paths, source invariants, and the archive are reviewed. Compilation and tests are not executed because this environment has no Rust toolchain. Phase 12 measurements and the Phase 8 runtime gate also remain pending.
