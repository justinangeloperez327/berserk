# Berserk roadmap

Berserk v0.8.0 is the current production-operations baseline. Development now moves toward stabilization and the 1.0 candidate surface.

## Completed

### v0.1.0 — Foundation

Application assembly, HTTP transport, routing, requests/responses, validation, database contracts and drivers, Claw ORM foundations, middleware/state, optional application components, testing infrastructure, and developer tooling.

### v0.2.0 — Typed application development

FormRequest extraction, request-scoped Claw operations, typed CRUD controllers, eager loading, transactions, pagination/resources, unified application errors, async action support, generators, and the foundation application.

### v0.3.0 — Developer experience

Concise request/response APIs, controller ergonomics, typed route parameters, resource routing, middleware ergonomics, typed state/configuration, CLI/application skeleton improvements, and a coherent CRUD example.

### v0.4.0 — Authentication and security

Authentication, authorization, secure password/token handling, concise route protection, rate limiting, CORS/security headers, and consistent 401/403/429 behavior.

### v0.5.0 — Application services

Explicit instance-scoped integration for cache, storage, events, jobs/queues, outbound HTTP, notifications, and mail-related application services.

### v0.6.0 — Database and Claw ORM

Expanded bound query predicates and model shortcuts, plus a complete migration rollback workflow while retaining explicit database execution and transaction boundaries.

### v0.7.0 — Testing

Expanded in-memory HTTP assertions and direct response inspection, plus richer event, job, and outbound HTTP test probes.

### v0.8.0 — Observability and production

Added programmatic readiness snapshots, operational metrics summaries, drainable in-memory logs, trace sampling inspection, and clearer deployment boundaries.

## Planned

### v0.9.0 — Stabilization

Reduce API churn, resolve known limitations where practical, strengthen documentation and compatibility guarantees, and prepare the public API for 1.0.

### v1.0.0 — Stable baseline

Establish the first stable Berserk public API and documented production baseline.

## Principles

Roadmap versions describe direction, not a promise that every proposed API will ship unchanged. Security, correctness, Rust safety, MSRV compatibility, and coherent developer experience take priority over copying another framework's syntax.
