# Architecture

## Target packages

All packages live under `crates/` with short folder names:

- `framework`: main import, App assembly, HTTP, routing, handlers, middleware, server, logging, health, metrics, trace propagation, and rate limiting.
- `core`: small shared foundations, state, configuration utilities, lifecycle contracts, core errors.
- `macros`: compile-time Berserk application derives, including the ergonomic `Model` derive; it contains no runtime framework state.
- `database`: execution interfaces, lexical request/job scope, query builder, migrations, and driver contracts.
- `claw`: typed models, guarded input conversions, relationships, eager loading, and pagination.
- `axe`: HTML templates, view values, escaping, and rendering; no database or ORM dependency.
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

Current package boundaries are architectural boundaries, not development phases. The `database` crate owns shared connection, transaction, statement, value, row, capability, migration, and driver contracts. Cache, storage, events, jobs, outbound HTTP, and notifications remain independent components and integrate through the main framework without depending on HTTP internals.

The `cli` remains independent of runtime application state; applications provide runtime-specific resources such as migration execution. The `testing` crate may depend on the framework, while production framework crates do not depend on testing. Hardening is cross-cutting rather than a separate runtime layer.

Folder names and Cargo package names are independent. Published package names use `berserk`, `berserk-*`, and `claw-orm`.

### Extensibility and control

Berserk uses inversion of control at framework boundaries: application code supplies behavior or implementations, and Berserk invokes them at the appropriate point in the request or application flow. This is distinct from a global dependency-injection container.

Prefer:

- small traits that describe a replaceable capability;
- application-owned implementations registered explicitly on an `App` instance;
- handler signatures that declare required framework-provided input;
- middleware and callbacks composed through explicit APIs;
- concrete defaults where a replaceable boundary is not needed.

Avoid introducing an interface for every concrete type. A trait is justified when the framework needs to call application-defined behavior, multiple implementations are useful, or testing/replacement requires a stable boundary. Application code should retain control over construction, configuration, ownership, and explicit escape hatches.


## Dependency direction

The main framework assembles components. Core must not depend on the main framework, HTTP implementation, or database drivers. Components use core only where needed; each owns its domain-specific errors. The `auth` and `openapi` crates do not depend on HTTP; the main framework adds their optional integrations. Re-exports and optional integrations must avoid circular dependencies. The testing package can depend on the framework without the framework depending on testing in production.

## Request execution

### Model presentation

Claw's `Model` trait remains the ordinary model contract. Berserk re-exports it
together with a compile-time `#[derive(Model)]` macro so application models can
declare table, primary-key, fillable, hidden, and optional column metadata on the
struct itself. The derive generates the same trait implementation that a manual
model would provide; it adds no runtime reflection. `model_fields!` and
handwritten `impl Model` remain supported for custom mappings. Visibility does
not change mass assignment or the specialized `PersistableModel` write mapping.

Query result sets and eager-loaded parents use `Collection<T>`. `Page<T>` owns
the same collection and retains its slice/vector accessors for compatibility.
The main framework converts visible model attributes into JSON or Axe values.
Neither Claw nor Axe depends on the other, and neither depends on the framework.
Only data passed to a view is exposed. No request, configuration, authentication,
or controller-local state is injected into templates.

Internal adapter marker types are inferred in the same way as controller
argument adapters. They avoid overlapping `ApiResource` blanket implementations.
Custom resources remain explicit; applications do not implement the bridge
traits. Scoped route binding remains a separate relationship-specific contract,
and create/update inputs remain distinct from hydrated models.

### Handler execution

The current stable API baseline began at [v1.0.0](v1.0.0.md); current 1.x release details are tracked in the versioned release notes. App installs a fresh lazy DatabaseScope around middleware and routing. The router installs the authenticated principal immediately around handler extraction and execution. Typed parameters resolve models, decode and sanitize forms, authorize them before semantic validation, and call the action. Errors remain typed until an explicit response boundary.

DatabaseScope uses checked exclusive connection borrowing through scoped-tls-hkt. Transactions install a borrowed connection view lexically and restore it during unwinding. No local unsafe code is required. Claw remains independent of HTTP: the framework supplies pagination context through database scope and maps domain errors at the HTTP boundary.

Optional async actions reuse the same extraction adapters. Their future is polled on one blocking worker while Tokio drives I/O, preserving lexical context without claiming that synchronous drivers are nonblocking. Independent tasks receive no implicit database or principal. This boundary is tested with concurrent current-thread-runtime callers and spawned tasks.
