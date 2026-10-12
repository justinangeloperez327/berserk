# Berserk architecture

Berserk provides a single application entry point while keeping its capabilities in separate Rust crates. You choose which facilities to enable; the framework does not require a particular folder hierarchy or a global service container.

## The application

An `App` owns its routes, configuration, and shared services. Register routes with `app.route()`, use middleware to compose request processing, and return typed `Response` values from handlers.

Requests carry the context for typed parameters, validated input, the authenticated user when configured, and application services. Services such as cache and storage are registered explicitly on the app instead of being discovered by file name.

## Components

| Component | Purpose |
| --- | --- |
| `berserk` | Application assembly, HTTP, routing, middleware, requests, and responses |
| `berserk-core` | Shared configuration, state, and lifecycle contracts |
| `berserk-validation` | Field and form validation |
| `berserk-database` | Database connections, bound queries, transactions, migrations, and SQL drivers |
| `claw-orm` | Models, queries, relationships, eager loading, and pagination |
| `berserk-axe` | HTML views and escaping |
| `berserk-auth` | Authentication and authorization primitives |
| `berserk-openapi` | OpenAPI definitions |
| `berserk-cache`, `berserk-storage` | Optional cache and file/object storage contracts |
| `berserk-events`, `berserk-jobs` | Events and background job facilities |
| `berserk-client`, `berserk-notifications` | Outbound HTTP and notifications |
| `berserk-cli`, `berserk-codegen`, `berserk-macros` | Developer commands, generators, and macros |
| `berserk-testing` | HTTP assertions, fakes, and test utilities |

## Request lifecycle

A typical HTTP request passes through route matching and middleware, optional authentication or authorization, typed input extraction and validation, the handler, and response rendering. Framework errors remain explicit until they become HTTP responses.

Claw builds queries before terminal operations execute them. Relationship loading is explicit, with batching to avoid per-record query patterns. Database connection and transaction scope are intentionally separate from other application services.

## Where to go next

Use the [public API guide](public-api.md) for application code, [project structure](project-structure.md) for generated modules, [database contract](database-contract.md) for SQL access, and [known limitations](known-limitations.md) for deployment boundaries.
