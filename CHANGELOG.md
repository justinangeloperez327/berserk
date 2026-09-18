# Changelog

Berserk follows the spirit of Keep a Changelog. Versioning and compatibility policy are defined in `docs/compatibility.md`.

## Unreleased

No documented changes after v0.3.0.

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
