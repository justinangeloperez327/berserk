# Standalone consumer

All application code is in src/main.rs. This package has its own workspace and explicit local framework dependency. It is not a member of the framework workspace. Keep the enclosing folder intact so the dependency path resolves; when moving the app elsewhere, update that path.

From the delivered root:

```sh
cargo run --manifest-path examples/minimal-api/Cargo.toml
```

Optional address:

```sh
cargo run --manifest-path examples/minimal-api/Cargo.toml -- 127.0.0.1:4000
```

The single-file application demonstrates inline/named/fallible handlers, raw byte responses and parameters. It has no database and DELETE /items/{id} is a response demonstration only: no item is actually stored or deleted. JSON, authentication and middleware are not yet included.

Try requests.http or `curl http://127.0.0.1:3000/users/42`. Expected body: User 42. For Windows use curl.exe if curl resolves to a shell alias. End the demo process with your terminal interrupt; it has no portable signal-driven graceful shutdown. Embedded usage of shutdown_handle remains available.

The consumer test starts the actual app on an ephemeral port and checks ten HTTP cases. Run it explicitly with the consumer manifest; root workspace tests do not include this independent package. Compilation and execution are unverified in the authoring environment.
