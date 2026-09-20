# Berserk roadmap

Berserk v1.0.0 is the current stable baseline. The 1.0 public API freeze and release audit are complete.

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

### v0.9.0 — Stabilization

Defined the preferred Berserk application surface, reduced default-prelude helper exposure, documented compatibility boundaries, repaired inherited formatting defects, and started the 1.0 freeze.

### v1.0.0 — Stable baseline

Established the first stable Berserk public API and documented production baseline.

## Principles

Roadmap versions describe direction, not a promise that every proposed API will ship unchanged. Security, correctness, Rust safety, MSRV compatibility, and coherent developer experience take priority over copying another framework's syntax.
