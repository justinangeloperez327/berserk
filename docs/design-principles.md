# Design principles

1. Laravel conventions take priority: consistent naming, fluent operations, integrated workflows.
2. Application architecture stays developer-controlled. Single-file applications are supported.
3. Use idiomatic Rust: explicit errors, typed boundaries, safe ownership, snake_case methods.
4. Preserve `where_` for the future query builder; use `or_where`, `where_in`, and `order_by` consistently.
5. Query chains construct queries; terminal methods execute them. Relationship loading is explicit.
6. Keep the initial HTTP foundation standard-library-only. Decide later driver, serialization, and cryptography integrations separately; do not invent cryptographic algorithms.
7. Add components only when their phase needs them. Do not scaffold the entire future system prematurely.
8. Validate configuration at startup. Bound network resources and expose understandable failure behavior.
9. Keep data models separate from public API representations by default.
10. Tests, documentation, and security review accompany implementation. The user owns publication.

## Conventions without mandatory layers

Controllers, services, repositories, and module folders are optional. Explicit registration determines behavior; filesystem paths do not. Generators may offer defaults but must not impose architecture.
