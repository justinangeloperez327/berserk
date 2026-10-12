# Design principles

Berserk combines approachable web-development conventions with idiomatic Rust:

1. **Explicit application state.** Routes, configuration, and services belong to an application instance; no global mutable registry is required.
2. **Developer-controlled organization.** Generated folders are conventions, not runtime requirements. A single-file API and larger modular applications are both valid.
3. **Typed boundaries.** Use explicit errors, validated inputs, and Rust types where they improve correctness.
4. **Predictable database access.** Query builders compose operations; terminal methods execute queries. SQL values stay bound separately from SQL text, and relationship loading is explicit.
5. **Composable services.** Authentication, storage, caching, events, jobs, and other facilities are opt-in through Cargo features.
6. **Bounded operation.** Configuration, network resources, and memory limits are explicit, with observable failures.
7. **Separation of concerns.** Domain models and public JSON or view representations may be kept separate.

Application APIs include `app.route()`, request-scoped access, typed configuration, Claw queries, and explicitly registered middleware. The `Arr` and `Str` utilities are available via explicit imports rather than the default prelude.

See [architecture](architecture.md) and [public APIs](public-api.md).
