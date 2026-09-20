# Architecture

## Target packages

All packages live under `crates/` with short folder names:

- `framework`: main import, App assembly, HTTP, routing, handlers, middleware, server, logging, health, metrics, trace propagation, and rate limiting.
- `core`: small shared foundations, state, configuration utilities, lifecycle contracts, core errors.
- `database`: execution interfaces, lexical request/job scope, query builder, migrations, and driver contracts.
- `claw`: typed models, guarded input conversions, relationships, eager loading, and pagination.
- `auth`: identity providers, password verification, sessions, principals, gates, policies, and optional HTTP integration.
- `openapi`: standalone schemas, operations, security definitions, validation, and OpenAPI 3.1 JSON generation.
- `cache`: backend-neutral byte-value cache operations, expiration, namespaces, and the bounded memory backend.
- `storage`: backend-neutral object operations, validated paths, and bounded memory/local-file backends.
- `events`: typed synchronous dispatch, ordered listeners, removal, and isolated failure reporting.
- `jobs`: bounded process-local queues, workers, retries, failed-job records, statistics, and recurring schedules.
- `client`: outbound HTTP contracts, validated request/response types, and the bounded plaintext TCP transport.
- `notifications`: mail and webhook messages, delivery policies, channel reports, and replaceable transports.
- `cli`: command parsing, safe generators, and application-provided migration command hooks.
- `testing`: in-memory HTTP assertions, fakes, recorders, and isolated temporary workspaces.
- `validation`: explicit field errors, validation rules, and sanitization helpers.

Beginning in Phase 13, `database` owns the shared connection, transaction, statement, value, row, capability, and error contracts. Phases 14–15 implement `database/src/drivers/postgres/`, `mysql/`, and `sqlite/`. They remain inside one database package, not three packages. Backend features are additive and disabled by default. The main `framework` package forwards those optional features. Backend-specific behavior remains explicit.

Folder names and Cargo package names are independent. Use a distinctive framework prefix for published package names; avoid naming the Rust crate itself `core` because Rust already supplies a core crate. Published names use `berserk`, `berserk-*`, and `claw-orm`.

Phase 22 keeps `cache` and `storage` independent of HTTP and database code. Applications may use the contracts directly or enable the main framework's `cache` and `storage` re-export features. Remote adapters must implement the same explicit contracts and map their failures into public component errors.

Phase 23 keeps domain events synchronous and background jobs explicit. `events` and `jobs` do not depend on HTTP, databases, cache, or storage. This prevents event publication from silently becoming remote I/O and allows durable brokers to be added later behind separate adapters and delivery guarantees.

Phase 24 keeps outbound protocols outside `core`. `notifications` depends on the `client` contract for webhooks, while neither component depends on the main framework. The framework only re-exports them through optional `client` and `notifications` features. TLS and SMTP implementations require opt-in adapters rather than weakening transport security to preserve a dependency-free default.

Phase 25 keeps `cli` independent of runtime application state. Applications supply a `MigrationExecutor` because only the application knows its database connection and registered migrations. `testing` depends on the main framework and selected component contracts, so it is never re-exported by or linked into the production framework; applications add it as a development dependency.

Phase 26 does not add a runtime crate. Hardening remains cross-cutting: crate roots forbid unsafe code, sensitive diagnostic output is metadata-only, the workspace declares an MSRV, and automated checks exercise the feature and consumer boundaries. Release automation stops before publication; artifact ownership and publication authority remain external to framework runtime code.

## Dependency direction

The main framework assembles components. Core must not depend on the main framework, HTTP implementation, or database drivers. Components use core only where needed; each owns its domain-specific errors. The `auth` and `openapi` crates do not depend on HTTP; the main framework adds their optional integrations. Re-exports and optional integrations must avoid circular dependencies. The testing package can depend on the framework without the framework depending on testing in production.

## Request execution

The current release contract is [v0.9.0](v0.9.0.md). App installs a fresh lazy DatabaseScope around middleware and routing. The router installs the authenticated principal immediately around handler extraction and execution. Typed parameters resolve models, decode/validate/authorize forms, and call the action. Errors remain typed until an explicit response boundary.

DatabaseScope uses checked exclusive connection borrowing through scoped-tls-hkt. Transactions install a borrowed connection view lexically and restore it during unwinding. No local unsafe code is required. Claw remains independent of HTTP: the framework supplies pagination context through database scope and maps domain errors at the HTTP boundary.

Optional async actions reuse the same extraction adapters. Their future is polled on one blocking worker while Tokio drives I/O, preserving lexical context without claiming that synchronous drivers are nonblocking. Independent tasks receive no implicit database or principal. This boundary is tested with concurrent current-thread-runtime callers and spawned tasks.
