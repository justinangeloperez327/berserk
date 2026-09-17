# Changelog

Berserk follows the spirit of Keep a Changelog. Versioning and compatibility policy are defined in `docs/compatibility.md`.

## Unreleased

No changes have been recorded after the `0.1.0` release candidate.

## 0.1.0 - release candidate

Initial public release candidate. No Berserk package has been published to crates.io yet.

### Added

- Instance-based `App` assembly with bounded Tokio/Hyper HTTP serving while keeping synchronous public request handlers.
- Routing with typed path parameters, named routes, reverse routing, scoped prefixes and middleware, REST resource registration, fallbacks, 404/405 handling, and `HEAD` fallback behavior.
- Request decoding for JSON, query strings, text, and bounded multipart input, including sanitize-then-validate controller input through `Validated<T>`.
- Driver-neutral database contracts, PostgreSQL/MySQL/SQLite adapters, migrations, bound query construction, request-scoped connections, and request transactions.
- Claw ORM model/query APIs including `where_`, `or_where`, `where_in`, `where_not_null`, `order_by`, terminal query operations, persistence helpers, and route-model binding.
- Optional auth, OpenAPI, cache, storage, events, jobs, outbound client, notifications, CLI, and testing components.
- Operational middleware and helpers for request IDs, logging, tracing, metrics, health checks, and rate limiting.
- Independent consumer example and clean external-consumer compilation checks.

### Security and reliability

- Bound SQL values remain separate from generated SQL text; raw SQL remains an explicit escape hatch.
- HTTP header validation rejects control bytes and non-ASCII/obs-text values outside the framework's documented strict text subset.
- Default diagnostics redact sensitive request targets, SQL/bindings, secrets, session tokens, and other protected values where documented.
- Storage paths reject traversal and ambiguous path forms; local storage uses bounded operations and collision-safe temporary writes.
- CI includes stable/MSRV builds, feature combinations, Windows/macOS compatibility, live PostgreSQL/MySQL/SQLite testing, dependency policy checks, fuzzing, package inspection, load/overload/shutdown testing, and prolonged soak evidence.

### Compatibility

- Candidate package version: `0.1.0`.
- MSRV: Rust 1.88.
- Tier-1 host: Linux x86_64 validated on Ubuntu 24.04 LTS.
- Development compatibility: Windows and macOS.
- Database support: PostgreSQL 15-18, MySQL 8.4 LTS, and bundled SQLite through the supported `rusqlite` path.
- Optional features are disabled by default.

### Known limitations

See `docs/known-limitations.md` for the release-candidate boundaries, including TLS, outbound HTTPS/SSRF policy, synchronous-handler shutdown behavior, session-store limits, cookie/CSRF policy, local-storage trust assumptions, request-transaction nesting, and platform/database scope.

### Upgrade notes

This is the first planned public release, so there is no migration path from an earlier published Berserk version. Existing path/git consumers should review `docs/upgrade-notes.md` because pre-release APIs and package identities were not previously compatibility-stable.

### Release status

The independent external security/API review, provenance/checksum preparation, rollback/yank planning, and explicit owner publication authorization remain separate release gates. No version has been released or published yet.
