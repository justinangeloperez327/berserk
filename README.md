# Berserk

Berserk is a Rust web framework focused on a clear developer experience, explicit behavior, type safety, and predictable performance.

> **Current version:** v0.3.0. Berserk is pre-1.0, so public APIs may still evolve. Rust 1.88 is the minimum supported Rust version (MSRV).

## Why Berserk?

- **Readable application APIs** — routing, requests, responses, validation, middleware, state, and database access are designed to work together.
- **Explicit execution** — query chains build operations; terminal methods perform database work.
- **Safe boundaries** — validated input, bound SQL values, bounded resources, and explicit errors.
- **Flexible structure** — start small and organize larger applications without mandatory folder discovery.
- **Feature-gated components** — enable only the subsystems an application needs.
- **Sync-first design** — synchronous application code remains the default; server and async capabilities are opt-in through Cargo features where applicable.

## Requirements

- Rust 1.88 or later
- Cargo
- PostgreSQL, MySQL, or SQLite only when the corresponding database feature is enabled

## Installation

When using the published package:

```toml
[dependencies]
berserk = "0.3"
```

Enable optional components as needed:

```toml
[dependencies]
berserk = { version = "0.3", features = ["postgres", "auth", "openapi"] }
```

Repository consumers can use the framework crate by path while developing Berserk itself.

## Quick start

```rust
use berserk::{response, App, Response, Result};

fn main() -> Result<()> {
    let mut app = App::new();

    {
        let mut route = app.route();
        route.get("/", || response().text("Hello from Berserk!"))?;
        route.get("/users/{id}", show_user)?;
    }

    app.listen("127.0.0.1:3000")
}

fn show_user(id: u64) -> Result<Response> {
    response().text(format!("User {id}"))
}
```

## Routing

Routes are registered through the application:

```rust
let mut route = app.route();

route.get("/health", health)?;
route.get("/users/{id}", users::show)?;
route.post("/users", users::store)?;
route.put("/users/{id}", users::update)?;
route.patch("/users/{id}", users::patch)?;
route.delete("/users/{id}", users::destroy)?;
```

Routes support typed parameters, names, reverse routing, scoped prefixes and middleware, groups, fallbacks, and resource registration. Invalid typed route parameters produce controlled client errors rather than invoking the handler.

## Requests and responses

v0.3.0 adds concise request input and response APIs:

```rust,ignore
let email = request.input("email")?;
let page = request.query("page")?;
let payload = request.json::<Payload>()?;
let input = request.validate::<CreateUserRequest>()?;

response().json(user)
response().status(201).json(user)
response().no_content()
```

FormRequest processing preserves Berserk's sanitize-then-validate lifecycle and authorization hooks.

## Middleware, state, and configuration

Middleware uses the existing request/response pipeline and can be applied globally, to a route, or to a scoped group. Application services and typed configuration are registered on an `App` instance and accessed through the request; Berserk does not require a process-global mutable application registry.

## Claw ORM

Claw provides model queries and request-scoped database access:

```rust,ignore
let users = User::where_("active", true)
    .where_not_null("email")
    .order_by("id", Direction::Asc)
    .get()?;

let user = User::find(id)?;
let page = User::query().paginate(20)?;
```

Query values are bound separately from SQL text. Explicit connection APIs remain available when request-scoped access is not appropriate.

## CLI

The CLI includes application and code-generation commands such as:

```sh
berserk new my-api
berserk serve
berserk make:model User
berserk make:controller UserController
berserk make:request CreateUserRequest
berserk make:middleware Audit
berserk make:resource UserResource
berserk make:policy UserPolicy
```

The generated application remains explicit: controllers, models, requests, middleware, configuration, and routes are ordinary Rust modules.

## Optional features

Major feature groups include `server`, `async`, `database`, `claw`, `postgres`, `mysql`, `sqlite`, `auth`, `openapi`, `cache`, `storage`, `events`, `jobs`, `client`, `notifications`, and `cli`.

See [docs/public-api.md](docs/public-api.md) for the current API contract and [docs/v0.3.0.md](docs/v0.3.0.md) for v0.3.0 details.

## Workspace

```text
crates/
├── framework       # Main Berserk API and HTTP application assembly
├── core            # Configuration, state, lifecycle, and shared foundations
├── database        # Connections, query builder, migrations, and SQL drivers
├── claw            # Claw ORM models, typed queries, and relationships
├── validation      # Validation contracts
├── auth            # Authentication and authorization
├── openapi         # OpenAPI generation
├── cache           # Cache contracts and implementations
├── storage         # Storage contracts and implementations
├── events          # Typed events
├── jobs            # Background jobs and scheduling
├── client          # Outbound HTTP
├── notifications   # Notifications
├── cli             # Developer commands and generators
└── testing         # Test helpers and fakes
```

## Development

Run the repository quality gate before merging release work:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
```

Additional security, dependency, database, fuzz, load, package, and release checks are defined by the repository workflows and [release checklist](docs/release-checklist.md).

## Documentation

- [Public API](docs/public-api.md)
- [Architecture](docs/architecture.md)
- [Design principles](docs/design-principles.md)
- [Compatibility and versioning](docs/compatibility.md)
- [Known limitations](docs/known-limitations.md)
- [Upgrade notes](docs/upgrade-notes.md)
- [Roadmap](docs/roadmap.md)
- [Support policy](docs/support-policy.md)
- [Security policy](SECURITY.md)

## Security

Berserk is pre-1.0 and should not be represented as independently security-certified. Review [SECURITY.md](SECURITY.md) and [known limitations](docs/known-limitations.md) before deployment.

## License

Berserk is licensed under the [MIT License](LICENSE).
