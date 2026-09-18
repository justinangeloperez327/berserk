# Berserk

Berserk is a Rust framework for building secure, maintainable web applications without forcing a specific project structure.

It combines familiar conventions and fluent APIs with Rust's explicit errors, type safety, predictable resource ownership, and performance.

> **Status:** v0.3.0 developer-experience work is in progress and has not been validated in this implementation pass. Berserk has not been published to crates.io. See the [v0.3.0 guide](docs/v0.3.0.md) for the new APIs and migration details.

## Why Berserk?

- **Application conventions** — readable APIs and features designed to work together.
- **Freedom of structure** — start with one file or organize a larger application however you prefer.
- **Explicit execution** — query chains build operations; terminal methods perform database work.
- **Safe boundaries** — validated HTTP input, bound SQL values, bounded resources, and explicit errors.
- **Optional components** — enable only the database drivers and application features you need.
- **Transparent behavior** — no hidden relationship queries, folder discovery, or mandatory architecture.

## Requirements

- Rust 1.88 or later
- Cargo
- PostgreSQL, MySQL, or SQLite only when its corresponding feature is enabled

## Installation

After Berserk is published, add it to your application's `Cargo.toml`:

```toml
[dependencies]
berserk = "0.3"
```

Enable optional components as needed:

```toml
[dependencies]
berserk = { version = "0.3", features = ["postgres", "auth", "openapi"] }
```

Until the package is published, use a local path dependency:

```toml
[dependencies]
berserk = { path = "../berserk/crates/framework" }
```

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

No controller, service layer, application folder, or macro is required.

## Routing

Routes are registered through an instance registrar borrowed from the application:

```rust
let mut route = app.route();

route.get("/health", health)?;
route.get("/users/{id}", users::show)?;
route.post("/users", users::store)?;
route.put("/users/{id}", users::update)?;
route.patch("/users/{id}", users::patch)?;
route.delete("/users/{id}", users::destroy)?;
```

Prefixes and middleware are scoped rather than global:

```rust
route
    .prefix("/api")
    .middleware(auth)
    .group(|route| {
        route.get("/users", users::index)?;
        route.get("/users/{id}", users::show)?;
        Ok(())
    })?;
```

Named routes preserve the existing verb APIs:

```rust
route
    .name("users.show")
    .get("/users/{id}", users::show)?;

let path = app.path_for("users.show", &[("id", "42")])?;
assert_eq!(path, "/users/42");
```

Route parameter values passed to `path_for` are percent-encoded as path segments. Missing parameters, unknown parameters, unknown names, and conflicting duplicate names return explicit routing errors.

### Resource routes

A full resource registers the conventional web REST surface:

```rust
route.resource("/users", UserController)?;
```

This creates:

```text
GET     /users             users.index
GET     /users/create      users.create
POST    /users             users.store
GET     /users/{id}        users.show
GET     /users/{id}/edit   users.edit
PUT     /users/{id}        users.update
PATCH   /users/{id}        users.update
DELETE  /users/{id}        users.destroy
```

The controller implements `ApiResourceController` for the API actions and `ResourceController` for the additional create/edit actions. Resource registration is atomic: if any generated route conflicts, none of the resource routes are committed.

For an API-only resource, omit the create/edit routes:

```rust
route.api_resource("/users", UserController)?;
```

When a resource is registered inside a prefix, generated names include the effective static path. For example:

```rust
route.prefix("/api").api_resource("/users", UserController)?;

let path = app.path_for("api.users.show", &[("id", "42")])?;
assert_eq!(path, "/api/users/42");
```

A fallback runs only when no route pattern matches, so method mismatches still return `405 Method Not Allowed`:

```rust
route.fallback(|request: Request| {
    Response::text(format!("No route for {}", request.path())).status(404)
})?;
```

Fallbacks can use route middleware but are intentionally root-scoped; registering a fallback beneath `prefix(...)` is rejected.

Handler signatures declare what Berserk should provide:

```rust
fn health() -> Response {
    Response::text("OK")
}

fn show(id: u64) -> Response {
    Response::text(format!("User {id}"))
}

fn update(id: u64, request: Request) -> Result<Response> {
    // Read the request only when the action needs it.
    todo!()
}
```

Multiple typed parameters are extracted in route-template order. A typed route parameter that cannot be parsed returns `400 Bad Request`. A controller expecting a different number of typed route parameters cannot be registered against the route.

## Typed controllers

With Claw enabled, actions work directly with models and validated FormRequest input:

```rust,ignore
fn update(mut user: User, input: UpdateUserRequest) -> Result<Response> {
    user.update(input)?;
    response().resource(user)
}
```

For `route.put("/users/{user}", update)`, Berserk parses the key, loads the model, sanitizes and validates the body, checks authorization, and invokes the action. Invalid keys return 400; missing models return 404. `route.crud("/users", Users)` registers a typed CrudController atomically.

## Scoped Claw ORM

```rust,ignore
app.database(database)?;

let users = User::where_("active", true)
    .where_not_null("email")
    .order_by("id", Direction::Asc)
    .get()?;

let user = Transaction::run(|| User::create(input))?;
let page = User::query().paginate(20)?;
```

One lazy database connection belongs to each request. Query construction performs no I/O; terminal methods execute SQL with bound values. Models declare their table, key, row mapping, and FILLABLE columns. Input types implement IntoInsert/IntoUpdate to select writable fields. Explicit connections remain available through `request.connection()` and Claw's `_on` methods.

`query.with(relation)` bulk loads typed relationships without hidden queries. Resource and ResourceCollection expose explicitly selected public fields. `ResourceCollection::page` includes pagination metadata. Unified application errors return JSON and redact internal details.

## Async actions and generators

Enable the optional `async` feature and use `route.get_async(...)` (or another verb) to await application I/O. Actions remain on a blocking worker so request scope survives awaits; synchronous database drivers still block that worker. Spawned tasks need their own scope. Dropping a response does not cancel an action that has started.

The CLI includes `new`, `serve`, `make:model`, `make:controller`, `make:request`, `make:middleware`, `make:resource`, and `make:policy`. New applications have explicit controllers, models, requests, middleware, configuration, and routes. See the [v0.3.0 guide](docs/v0.3.0.md), [migration notes](docs/upgrade-notes.md), and [foundation CRUD application](examples/foundation/README.md).

## Optional features

| Feature         | Purpose                                      |
| --------------- | -------------------------------------------- |
| `server`        | Tokio/Hyper serving; enabled by default       |
| `async`         | Optional async application actions          |
| `database`      | Driver-neutral database contracts            |
| `claw`          | Claw ORM model and relationship layer        |
| `postgres`      | PostgreSQL driver                            |
| `mysql`         | MySQL driver                                 |
| `sqlite`        | SQLite driver                                |
| `auth`          | Authentication and authorization contracts   |
| `openapi`       | OpenAPI document generation                  |
| `cache`         | Cache contracts and in-memory cache          |
| `storage`       | Storage contracts and local/memory storage   |
| `events`        | Typed application events                     |
| `jobs`          | Bounded background jobs and scheduling       |
| `client`        | Outbound HTTP client contracts               |
| `notifications` | Mail and webhook notifications               |
| `cli`           | Optional development commands and generators |

No optional feature is enabled by default.

## Workspace

The repository separates runtime responsibilities into focused crates:

```text
crates/
├── framework       # Main Berserk API and HTTP application assembly
├── core            # Configuration, state, lifecycle, and shared foundations
├── database        # Connections, query builder, migrations, and SQL drivers
├── claw            # Claw ORM models, typed queries, and relationships
├── validation      # Reusable validation contracts
├── auth            # Authentication and authorization
├── openapi         # API contracts and OpenAPI generation
├── cache           # Cache contracts and memory implementation
├── storage         # Storage contracts and local/memory implementations
├── events          # Typed event dispatch
├── jobs            # Background jobs, retries, failures, and scheduling
├── client          # Outbound HTTP
├── notifications   # Mail and webhooks
├── cli             # Developer commands and generators
└── testing         # Assertions, fakes, recorders, and test workspaces
```

Short folder names are intentional. Published package names will use the Berserk namespace.

## Development checks

Run the complete local quality gate:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
cargo audit
cargo deny check
```

Verify the minimum supported Rust version separately:

```sh
cargo +1.88 check --workspace --all-targets --all-features
cargo +1.88 test --workspace --all-features
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution workflow and [docs/release-checklist.md](docs/release-checklist.md) for the full release gate.

## Security

Berserk is not yet independently security-audited. Do not describe it as production-ready until the documented compilation, test, dependency, fuzzing, live-database, load, soak, and review gates pass.

Please report vulnerabilities privately according to [SECURITY.md](SECURITY.md). Do not include credentials, private data, or exploit details in a public issue.

## Documentation

Design contracts, architecture notes, acceptance checks, compatibility policy, and component documentation are available in the [docs](docs) directory.

## License

Berserk is licensed under the [MIT License](LICENSE).
