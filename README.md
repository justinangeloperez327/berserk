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

A typed route parameter that cannot be parsed returns `400 Bad Request`. A controller expecting one typed route parameter cannot be registered against a route with a different parameter count.

The router provides static-route precedence, path parameters, `404`, `405`, automatic `HEAD` fallback to `GET`, scoped middleware, nested prefixes, and atomic route groups.

## Fluent database queries

Berserk keeps Laravel-style readability while making database execution visible:

```rust
let users = User::query()
    .where_("active", "=", true)
    .where_not_null("email")
    .order_by("created_at", Direction::Desc)
    .limit(20)
    .get(&mut connection)?;
```

Methods such as `where_`, `or_where`, `where_in`, `where_not_null`, and `order_by` build the query. Terminal methods such as `get`, `first`, and `paginate` execute it.

SQL values remain separate from SQL text through bound parameters. Raw SQL remains available as an explicit escape hatch.

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
