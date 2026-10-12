# Berserk

**Berserk is a Rust web framework for building APIs, web applications, and backend services.**

It combines familiar web-development conventions with Rust's type safety and explicit execution model. Applications can start with a single route and grow into projects with controllers, models, middleware, validation, authentication, database access, and background services.

## What Berserk provides

- **HTTP and routing:** typed route parameters, named routes, middleware, controllers, request handling, and responses.
- **Validation and security:** validated request data, authentication, authorization, and configurable HTTP protections.
- **Databases and ORM:** PostgreSQL, MySQL, and SQLite drivers, migrations, query builders, and Claw ORM relationships with explicit eager loading.
- **HTML views:** the Axe template engine.
- **Application services:** optional cache, storage, events, jobs, outbound HTTP, and notifications.
- **Developer tools:** a CLI, project generators, testing helpers, and operational health, logging, tracing, and metrics primitives.

Components are selected with Cargo features; your Rust modules and application structure remain under your control.

## Get started

**Requirements:** Rust 1.88 or newer, with Cargo.

~~~sh
cargo new hello-berserk
cd hello-berserk
~~~

Add Berserk from its v1.0.0 GitHub release:

~~~toml
[dependencies]
berserk = { git = "https://github.com/justinangeloperez327/berserk", tag = "v1.0.0" }
~~~

Enable optional components when needed, for example:

~~~toml
[dependencies]
berserk = { git = "https://github.com/justinangeloperez327/berserk", tag = "v1.0.0", features = ["sqlite", "claw", "auth"] }
~~~

Create `src/main.rs`:

~~~rust
use berserk::{response, App, Response, Result};

fn main() -> Result<()> {
    let mut app = App::new();

    {
        let mut routes = app.route();
        routes.get("/", || response().text("Hello from Berserk!"))?;
        routes.get("/users/{id}", show_user)?;
    }

    app.listen("127.0.0.1:3000")
}

fn show_user(id: u64) -> Result<Response> {
    response().text(format!("User {id}"))
}
~~~

Run `cargo run`, then open `http://127.0.0.1:3000`.

## Berserk CLI

Windows users can install the CLI with the official [v1.0.0 Windows installer](https://github.com/justinangeloperez327/berserk/releases/download/v1.0.0/BerserkSetup-1.0.0-windows-x64.exe). It adds the `berserk` command to the user PATH. Installing the CLI does not install Rust or the Microsoft C++ build tools required to compile applications.

~~~powershell
berserk new my-app
cd my-app
berserk serve
~~~

You can also generate controllers, models, migrations, requests, resources, policies, and middleware. See the [CLI and generators guide](docs/codegen.md) and [Windows installation guide](docs/windows-installation.md).

## Documentation

Start with the [documentation index](docs/index.md). The main guides cover [public APIs](docs/public-api.md), [project structure](docs/project-structure.md), [migrations](docs/migrations.md), [Claw relationships](docs/relationships.md), [Axe views](docs/views.md), and [supported environments](docs/support-policy.md).

An executable [minimal API example](examples/minimal-api/README.md) and a larger [Foundation example](examples/foundation/README.md) demonstrate application code.

## Compatibility and security

Berserk v1.0.0 is the initial stable API baseline, with Rust 1.88 as its MSRV. See [compatibility](docs/compatibility.md), [known limitations](docs/known-limitations.md), and the [security policy](SECURITY.md) when evaluating production use. Berserk does not claim an independent security audit or certification.

## License

[MIT](LICENSE).
