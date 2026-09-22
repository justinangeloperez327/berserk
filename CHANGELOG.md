# Changelog

Berserk follows the spirit of Keep a Changelog. Versioning and compatibility policy are defined in `docs/compatibility.md`.

Berserk has not yet been published publicly. The entries below `1.0.0` describe development milestones that are being consolidated into the first public v1.0.0 release rather than previously published package releases.

## Unreleased

### Changed

- Consolidated migration, Claw relationship, and request-plumbing work into the first public v1.0.0 release target.
- Documentation now distinguishes internal development milestones from public releases.
- Development branches are named for their work; version numbers are reserved for actual release preparation.

### Architecture

- Extensibility work is centered on Rust-native inversion of control: explicit replaceable boundaries and application-owned implementations rather than a global dependency-injection container.
- Controller scaffolding now has one source of truth in `berserk-codegen`; the CLI writes generated source instead of maintaining a separate controller template.
- FormRequest scaffolding and optional model-bound Claw write mappings now use the same `berserk-codegen` boundary; the CLI request template is removed.
- Migration source generation now lives in `berserk-codegen`; the CLI owns timestamps and filesystem placement while schema source generation has one reusable implementation.
- Model scaffolding now uses a reusable `ModelSpec`/`FieldSpec` source representation in `berserk-codegen`; the CLI model template is removed while the existing `#[derive(Model)]` runtime contract remains unchanged.
- Application route scaffolding now uses `RoutesSpec`/`RouteSpec` in `berserk-codegen`; generated projects no longer depend on a CLI-owned routes template and CRUD registrations compile to explicit `app.route().crud(...)` calls.
- The complete conventional `berserk new` skeleton now comes from `ApplicationSpec`/`ApplicationFile` in `berserk-codegen`; the CLI main/config templates and inline module-source assembly are removed while filesystem safety remains in the CLI.
- Middleware, API-resource, and policy scaffolds now use `berserk-codegen`; the final CLI-owned Rust templates are removed, leaving source generation and CLI filesystem orchestration as separate boundaries.
- The first public release now has an explicit maturity gate covering coordinated CRUD generation, Claw eager-loading ergonomics, paginated presentation, bounded eager loading, Axe production validation, a reference application, independent review, and exact-commit release evidence.
- `ModuleSpec` now coordinates model, request, controller, migration, and route conventions, and `berserk make:crud <Name>` generates and registers a coherent CRUD slice from that shared specification.
- Generated migrations are registered through the application's migration module index so generator verification compiles migration source rather than leaving it as an unreferenced file.
- Named Claw eager queries now preserve normal parent-query composition after `.with(...)`, including filters, list/range/null predicates, ordering, limits, offsets, scopes, statement inspection, and pagination.
- Named eager-loaded pages now have first-class JSON and Axe presentation with `data` plus pagination `meta`, while retaining direct page and relation access in Claw.

## 1.0.0 — first public release target

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
