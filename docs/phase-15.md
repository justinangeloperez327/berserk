# Phase 15 — MySQL and SQLite drivers

Phase 15 completes the first execution layer for all three planned databases.

## MySQL

- Uses `mysql` 28.0.2 with prepared binary-protocol queries.
- Uses the minimal Rust dependency set and Rustls ring TLS feature.
- Supports NULL, signed/unsigned integers, finite floats, text, and binary values.
- Preserves server error numbers and classifies common constraint, lock-timeout, and deadlock failures.
- Supports transaction access mode and native last-insert IDs.
- Live tests are enabled by setting `FRAMEWORK_MYSQL_TEST_URL`.

MySQL decimal, date/time, and JSON values return explicit decode errors until precision-preserving framework types exist. Binary column flags determine whether byte results remain bytes; invalid text encoding is an error.

## SQLite

- Uses `rusqlite` 0.40.2 with its bundled SQLite build for portability.
- Supports file-backed and in-memory connections.
- Supports NULL, signed integers, finite floats, text, and blobs.
- Maps SQLite error codes and preserves extended numeric codes.
- Includes always-runnable in-memory CRUD and rollback test sources.

SQLite stores integers as signed 64-bit values, so larger `u64` inputs fail before execution. The shared read-only transaction option is rejected because SQLite's `query_only` pragma is connection-scoped rather than a reliable transaction-scoped guarantee. SQLite generated IDs are not inferred from arbitrary statements; future higher-level insert APIs can expose them without returning stale connection state.

## Feature selection

```toml
framework = { version = "0.1", features = ["postgres"] }
framework = { version = "0.1", features = ["mysql"] }
framework = { version = "0.1", features = ["sqlite"] }
```

Features are additive, so applications may enable several. Driver dependencies remain absent from default builds.

## Verification status

All project files, manifests, feature wiring, module paths, and test sources were reviewed. Compilation, dependency resolution, and live tests remain pending because no Rust toolchain is available. SQLite test sources require no external server once compiled. Phase 12 measurements and earlier runtime gates also remain pending.
