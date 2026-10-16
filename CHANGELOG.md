# Changelog

## Unreleased

- Bound Axe directive nesting and reject Windows-special storage paths on all platforms.
- Release migration locks before propagating application panics; avoid scheduler factory deadlocks and join all workers during shutdown.
- Add fallible trace generation, reject overflowing runtime capacities/timeouts, fix out-of-range negative string indices, and remove an unnecessary memory-storage payload clone.
- Reject duplicate derived model columns and malformed bracketed IPv6 URL authorities.
- Update locked rustls to 0.23.46 to address four RustSec advisories.
- Run live database and dependency-policy workflows for relevant direct pushes to main.

## 1.0.0

Berserk's initial public stable release establishes the Rust web framework's application-facing APIs.

- HTTP server, routing, typed parameters, middleware, request and response helpers, controllers, and validation.
- PostgreSQL, MySQL, and SQLite support, driver-neutral migrations, bound queries, and Claw ORM models and relationships.
- Authentication and authorization, Axe views, and optional OpenAPI, caching, storage, events, jobs, outbound HTTP, and notifications.
- CLI application generation and scaffolding, Windows CLI distribution, testing utilities, and operational observability facilities.
- Rust 1.88 minimum supported version and semantic-versioned public API.

For current usage, see the [documentation index](docs/index.md), [compatibility](docs/compatibility.md), and [known limitations](docs/known-limitations.md).
