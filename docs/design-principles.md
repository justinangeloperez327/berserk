# Design principles

1. Berserk keeps its own identity. Learn from proven framework ideas, but do not copy another framework's public API merely for familiarity.
2. Application architecture stays developer-controlled. Single-file applications are supported.
3. Use idiomatic Rust: explicit errors, typed boundaries, safe ownership, snake_case methods, and compile-time contracts where they improve clarity.
4. Prefer small expressive APIs over aliases, facades, magic discovery, or runtime string registries.
5. Query chains construct queries; terminal methods execute them. Values remain bound separately from SQL text, and relationship loading stays explicit.
6. Keep synchronous application code straightforward. Async/server infrastructure remains opt-in where it provides concrete value.
7. Add components only when their release needs them. Do not scaffold the entire future system prematurely.
8. Validate configuration at startup. Bound network and memory resources and expose understandable failure behavior.
9. Keep data models separate from public API representations by default.
10. Tests, documentation, security review, and compatibility notes accompany implementation. The project owner controls publication.

## Preferred public surface

Berserk's preferred application-facing APIs are instance- and request-scoped:

- register routes through `app.route()`;
- read request context through `Request`;
- configure authentication once and use route scopes such as `.auth()` and `.can(...)`;
- register application services explicitly on `App`;
- use Claw query builders with bound values;
- opt into operational middleware explicitly.

Compatibility shortcuts can remain during pre-1.0 development, but they do not define Berserk's long-term identity.

## Utilities

`Arr` and `Str` are optional explicit utilities. They are not part of the default prelude and are not architectural requirements.

## Conventions without mandatory layers

Controllers, services, repositories, and module folders are optional. Explicit registration determines behavior; filesystem paths do not. Generators may offer defaults but must not impose architecture.
