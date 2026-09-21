# Project structure

Berserk provides a project convention, not a required application architecture.

A newly generated application uses:

```text
src/
├── app/
│   ├── controllers/
│   ├── models/
│   ├── validations/
│   ├── middleware/
│   ├── resources/
│   ├── policies/
│   └── routes.rs
├── database/
│   └── migrations/
├── config/
└── main.rs
```

The folders are generator defaults. Berserk runtime behavior does not discover controllers, models, validations, routes, or services by filesystem location.

## Custom structures

Applications may reorganize their Rust modules. A feature-oriented application can use:

```text
src/
├── app/
│   └── modules/
│       ├── users/
│       ├── orders/
│       └── inventory/
├── database/
│   └── migrations/
├── config/
└── main.rs
```

A smaller application may use a flatter structure. Both are valid because Rust module declarations and explicit registration are authoritative.

For example, conventional routes can expose a registration function:

```rust
app::routes::register(&mut app)?;
```

A modular application can instead compose modules explicitly:

```rust
app::modules::users::register(&mut app)?;
app::modules::orders::register(&mut app)?;
```

Berserk does not scan folders, infer runtime components from paths, or require a particular module hierarchy.

## Generator behavior

CLI generators use the conventional folders as their default destinations:

- models → `app/models`
- controllers → `app/controllers`
- requests/FormRequests → `app/validations`
- middleware → `app/middleware`
- resources → `app/resources`
- policies → `app/policies`
- migrations → `database/migrations`

These paths are scaffolding choices, not framework contracts. Generated source is ordinary Rust and may be moved or reorganized by the application.

## Design rule

**Convention provides the default. Rust modules provide the freedom.**

Application structure remains application-owned. Berserk owns framework execution and explicit integration boundaries, not the application's filesystem.
