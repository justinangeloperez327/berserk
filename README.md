# Berserk

Berserk is a Laravel-inspired Rust framework for building secure, maintainable web applications without forcing a specific project structure.

It combines familiar conventions and fluent APIs with Rust's explicit errors, type safety, predictable resource ownership, and performance.

> **Status:** Berserk is under active development and has not been published to crates.io. The public API may change before the first stable release.

## Why Berserk?

- **Laravel-inspired conventions** — readable APIs and features designed to work together.
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
berserk = "0.1.0"
```

Enable optional components as needed:

```toml
[dependencies]
berserk = { version = "0.1.0", features = ["postgres", "auth", "openapi"] }
```

Until the package is published, use a local path dependency after the crates have been renamed to the Berserk package namespace:

```toml
[dependencies]
berserk = { path = "../berserk/crates/framework" }
```

## Quick start

```rust
use berserk::{App, Response, Result};

fn main() -> Result<()> {
    let mut app = App::new();

    {
        let mut route = app.route();
        route.get("/", || Response::text("Hello from Berserk!"))?;
        route.get("/users/{id}", show_user)?;
    }

    app.listen("127.0.0.1:3000")
}

fn show_user(id: u64) -> Response {
    Response::text(format!("User {id}"))
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

With Claw enabled, route models and validated input can be injected directly:

```rust
fn update(
    mut user: User,
    input: Validated<UpdateUser>,
    request: Request,
) -> Result<Response> {
    user.name = input.name.clone();

    let mut connection = request.connection()?;
    user.save(&mut *connection)?;

    Ok(Response::empty().status(204))
}
```

For `route.put("/users/{user}", update)`, Berserk parses the model key, loads the model through Claw, returns `404` when the model is missing, then decodes, sanitizes, and validates the request body before invoking the controller.

The router provides static-route precedence, path parameters, named paths, REST resources, `404`, `405`, automatic `HEAD` fallback to `GET`, scoped middleware, nested prefixes, atomic route groups, and a root fallback.

## Fluent database queries

Berserk keeps Laravel-style readability while making database execution visible. `User::query()` remains the canonical query-builder entry point:

```rust
let users = User::query()
    .where_("active", "=", true)
    .where_not_null("email")
    .order_by("created_at", Direction::Desc)
    .limit(20)
    .get(&mut connection)?;
```

Convenience entry points remain available when a query starts with a known predicate:

```rust
let users = User::where_("active", "=", true)
    .order_by("name", Direction::Asc)
    .get(&mut connection)?;
```

Methods such as `where_`, `or_where`, `where_in`, `where_not_null`, and `order_by` build the query. Terminal methods such as `get`, `first`, `count`, `exists`, `update`, `delete`, and `paginate` execute it.

SQL values remain separate from SQL text through bound parameters. Raw SQL remains available as an explicit escape hatch.

## Claw model lifecycle

Claw keeps row decoding and write serialization separate. Every model implements `Model`; models that want automatic `save()` explicitly opt into `PersistableModel` and declare the columns they allow Claw to write:

```rust
impl PersistableModel for User {
    fn values_for_save(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("name", self.name.clone().into()),
            ("active", self.active.into()),
        ]
    }
}
```

The primary key is always used as the update filter and is rejected if it is included in `values_for_save`:

```rust
user.name = "Grace".into();
user.save(&mut connection)?;
```

Explicit instance mutation is also available when only selected columns should be written:

```rust
user.update(
    &mut connection,
    [("name", Value::from("Grace"))],
)?;

user.delete(&mut connection)?;
```

`update` does not silently rewrite the Rust struct. Use `fresh()` to fetch a new copy or `refresh()` to replace the current model from the database:

```rust
let fresh_user = user.fresh(&mut connection)?;

if user.refresh(&mut connection)? {
    // `user` now contains the current database row.
}
```

Database access can be registered once on the application and acquired explicitly from a request:

```rust
app.database(database)?;

let mut connection = request.connection()?;
```

`request.connection()` lazily acquires one connection for that request and reuses it across non-overlapping borrows. This keeps ownership visible while allowing route-model binding and controller persistence to share the same acquired connection. Pooling strategy remains behind the `Database` acquisition boundary.

### Request transactions

Use the request-scoped connection for atomic database work without changing the normal Claw or query-builder APIs:

```rust
request.transaction(TransactionOptions::default(), |connection| {
    user.save(connection)?;
    audit.save(connection)?;

    Ok(())
})?;
```

The transaction reuses the request's existing connection. The closure receives a connection-compatible mutable reference, so Claw operations and query-builder terminal methods use the same syntax they use outside a transaction.

Berserk commits when the closure returns `Ok` and rolls back when it returns `Err`. When rollback succeeds, the original application error is returned. A commit or rollback failure is surfaced as a database transaction error because the final transaction state is uncertain.

Read-only transactions can be requested explicitly:

```rust
request.transaction(
    TransactionOptions { read_only: true },
    |connection| {
        let users = User::query().get(connection)?;
        Ok(users)
    },
)?;
```

Driver capabilities still apply; a driver may reject an unsupported transaction option. Nested transactions are not currently supported through the request transaction view.

## Optional features

| Feature         | Purpose                                      |
| --------------- | -------------------------------------------- |
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
