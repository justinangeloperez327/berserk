# Changelog

Berserk follows the spirit of Keep a Changelog. Versioning and compatibility policy are defined in `docs/compatibility.md`.

## Unreleased

## 1.2.0 (release candidate)

### Added

- Parameterized many-to-many `attach`, `attach_many`, `detach`, `detach_many`, `detach_all`, and transactional `sync`, with explicit connection variants and `SyncResult`.
- `query_for` returning ordinary `ModelQuery` for all four relationships; parent constraints remain effective across OR filters through additive `Query::constrain_in`.
- Scoped `Relationship::load`, comprehensive SQLite/error/query-count tests, and live PostgreSQL/MySQL relationship contracts in CI.
- A focused SQLite relationship example and complete relationship guide.

### Fixed

- Signed/unsigned driver key comparisons in eager loading and synchronization.
- Restored stable 1.0 explicit `load(connection, ...)` methods as deprecated forwarders to `load_on`.
- Malformed NULL pivot keys now report Decode errors; duplicate pivot rows and shared related models are preserved.
- Synchronized workspace packages, internal dependency requirements, and lockfiles at 1.2.0; release planning now rejects stale internal requirements.

### Compatibility

- Rust 1.88 remains the MSRV. HasOne retains RelatedSet; no Clone requirement, new error hierarchy, declaration macros, or external dependencies are added.
- Existing NULL/dangling parent keys remain skipped during batch loading, and missing related records remain omitted.
- Sync reconciles membership, retains duplicates for unchanged keys, and rejects nested transactions. See `docs/relationships.md`.

## 1.1 migration work (included in 1.2)

### Added

- Driver-neutral migration DSL with `Table`, `Column`, indexes, foreign keys, composite primary keys, and explicit table alteration operations.
- PostgreSQL, MySQL, and SQLite migration SQL compilation with explicit portability errors for unsupported operations.
- `MigrationPlan` for composing schema operations into the existing `Migration` contract without breaking the v1.0 trait.
- Migration generator output using the typed DSL for conventional `create_*_table` migrations.
- Migration DSL, compiler, constraint, alteration, and plan test coverage.
- Portable `CURRENT_TIMESTAMP` column defaults, dry-run planning, and migration reset command support.
- Portable identifier validation and duplicate composite-key/index constraint checks.
- Check and explicit unique constraints, enum-like columns, generated stored columns, schema comments, and custom SQL type escape hatches.
- Alter-table index renames, default changes, check/unique constraint changes, and explicit SQLite table rebuild migrations.
- Serialized migration execution with PostgreSQL/MySQL advisory locking and SQLite immediate write locking.
- Transactional migration failure recovery on databases that support transactional DDL.

### Changed

- Migration generation now favors typed schema definitions over handwritten SQL for conventional create-table migrations.

See `docs/migrations.md`.

## 1.0.0

### Changed

- Froze route registration on the `app.route()` registrar and removed direct application verb compatibility shortcuts.
- Removed `Request::principal()`; request-scoped authentication now uses `Request::user()`, `Request::can()`, and `Request::authorize()`.
- Request logging now preserves propagated HTTP error status codes.
- Metrics count propagated errors as failures only when they resolve to 5xx server errors.
- Hardened generated-project verification against the local framework contract.
- Aligned all workspace package and internal dependency versions to v1.0.0.

### Compatibility

- Rust 1.88 remains the MSRV.
- v1.0.0 establishes the stable public API baseline under semantic versioning.
- Breaking public API changes require a new major release.

See `docs/v1.0.0.md`, `docs/compatibility.md`, and `docs/upgrade-notes.md`.

## 0.9.0

### Changed

- Began the 1.0 stabilization window and documented the preferred instance- and request-scoped API surface.
- Removed `Arr` and `Str` from the default prelude while keeping them available as explicit root utilities.
- Marked request-local authentication and route registration as the preferred application style over compatibility facades and shortcuts.
- Aligned workspace package metadata to v0.9.0 and made package CI derive the synchronized release version dynamically.
- Applied canonical formatting fixes inherited from the v0.8 merge.

### Added

- A public-surface contract test covering the preferred routing/response API and explicit support-utility imports.
- A v0.9 stabilization and compatibility guide.

### Compatibility

- Rust 1.88 remains the MSRV.
- Compatibility APIs remain available during the pre-1.0 transition unless otherwise documented.
- v0.9.x should favor fixes and API clarification over new public concepts.

See `docs/v0.9.0.md`, `docs/compatibility.md`, and `docs/upgrade-notes.md`.

## 0.8.0

### Added

- Programmatic health snapshots with per-check status and aggregate readiness.
- Metrics summary helpers for completed requests, average duration, and simple health state.
- Drain/clear operations for the in-memory log sink.
- Trace-context flag and sampling inspection.

### Changed

- HTTP readiness rendering now uses the same programmatic health snapshot model.
- Workspace package metadata and package validation are aligned to v0.8.0.

### Compatibility

- Rust 1.88 remains the MSRV.
- Existing health, logging, metrics, and tracing APIs remain available.

See `docs/v0.8.0.md`.

## 0.7.0

### Added

- OPTIONS support in the in-memory HTTP test client.
- Direct status, header, body, and text inspection on `TestResponse`.
- Redirect, client-error, server-error, text-contains, and missing-JSON-path assertions.
- Event-recorder last/clear helpers, job-probe last-attempt inspection, and outbound-request count/drain helpers.

### Changed

- Workspace package metadata and package validation are aligned to v0.7.0.

### Compatibility

- Rust 1.88 remains the MSRV.
- Existing test APIs remain available; the release is additive for application tests.

See `docs/v0.7.0.md`.

## 0.6.0

### Added

- Bound `BETWEEN` and `NOT BETWEEN` predicates in the database query builder and Claw ORM.
- OR variants for Claw list and null predicates.
- Model-level shortcuts for range, list, and null query variants.
- `MigrationRunner::rollback_all` for deterministic newest-to-oldest migration resets.

### Changed

- Workspace package metadata and package validation are aligned to v0.6.0.

### Compatibility

- Rust 1.88 remains the MSRV.
- Query values remain parameter-bound; the new range predicates do not interpolate values into SQL.

See `docs/v0.6.0.md`.

## 0.5.0

### Added

- Explicit application registration for cache, storage, events, jobs, outbound HTTP, and notifications.
- Request accessors for configured application services without a global service locator.
- v0.4.0 and v0.5.0 release guides.

### Changed

- Application-service integration now uses typed, instance-scoped state rather than requiring handlers to manually retrieve generic state.
- Workspace package metadata and package validation are aligned to v0.5.0.
- The authentication refactor's clippy warning is corrected.

### Compatibility

- Rust 1.88 remains the MSRV.
- Existing generic `App::state`, `Request::state`, and `Request::shared` APIs remain available.

See `docs/v0.5.0.md` and `docs/upgrade-notes.md`.

## 0.4.0

### Added

- Secure session and personal/API token primitives with expiry, revocation, and abilities.
- Concise configured authentication with `app.auth(...)` and route-level `.auth()`, `.guest()`, and `.can(...)`.
- `Request::user()`, `Request::can(...)`, and resource authorization helpers.
- Explicit CORS and security-header middleware.
- Security coverage for malformed credentials, token redaction, CORS validation, and authorization behavior.

### Changed

- Authentication internals remain available, but the primary route API hides guard plumbing for normal application code.
- Authorization consistently distinguishes unauthenticated 401 responses from authenticated 403 denials.

### Compatibility

- Rust 1.88 remains the MSRV.
- CSRF is not a first-class v0.4 feature because Berserk does not provide browser cookie authentication as a first-class mode.

See `docs/v0.4.0.md`.

## 0.3.0

### Added

- Concise response factory APIs for JSON, text, empty responses, status/header composition, and redirects.
- Request helpers for named input, query values, typed JSON decoding, and FormRequest validation.
- Developer-facing controller signatures with typed route parameters.
- Resource and API-resource routing improvements.
- Middleware ergonomics for routes and groups.
- Typed application state and configuration access.
- CLI application skeleton improvements and middleware generation.
- Updated foundation CRUD example and v0.3.0 developer documentation.

### Changed

- Network serving is isolated behind the `server` feature while in-memory application handling remains available without it.
- Response factory terminals return `Result<Response>` so completed responses can be validated.
- Generated applications target the v0.3 API surface.

### Compatibility

- MSRV remains Rust 1.88.
- Berserk remains pre-1.0; breaking changes may occur in later minor releases with migration guidance.

See `docs/v0.3.0.md` and `docs/upgrade-notes.md`.

## 0.2.0

### Added

- Direct FormRequest parameters with sanitize, validation, request-aware checks, and authorization.
- Request-scoped Claw terminals and explicit `_on` connection escape hatches.
- Typed eager loading, scoped transactions, pagination, and paginated resource metadata.
- Unified JSON application errors and request-local authentication context.
- Typed `CrudController` routes and optional async actions.
- Controller, request, model, resource, and policy generators.
- SQLite foundation application and coordinated workspace release metadata.

### Compatibility

Rust 1.88 remains the MSRV. See `docs/v0.2.0.md` and `docs/upgrade-notes.md`.

## 0.1.0

Initial Berserk release-candidate baseline.

### Added

- Instance-based `App` assembly and HTTP serving.
- Typed routing, named routes, reverse routing, middleware, resources, fallbacks, and 404/405 handling.
- Request decoding, JSON, multipart input, validation, and `Validated<T>`.
- Driver-neutral database contracts and PostgreSQL/MySQL/SQLite adapters.
- Claw ORM query, model, relationship, persistence, and route-model-binding APIs.
- Auth, OpenAPI, cache, storage, events, jobs, outbound client, notifications, CLI, and testing components.
- Operational middleware, CI, package checks, fuzzing, and load-test infrastructure.

See `docs/known-limitations.md` for current boundaries.
