# Berserk CLI and code generation

The `berserk` command creates an application and generates conventional Rust source files. Generated code is editable Rust; the framework does not depend on runtime folder scanning.

## Create and serve an application

~~~sh
berserk new my-app
cd my-app
berserk serve
~~~

On Windows, install the CLI using the [Windows guide](windows-installation.md). A working Rust toolchain is needed to compile generated applications.

## Generate application code

~~~sh
berserk make:model User
berserk make:controller UserController
berserk make:controller UserController --resource --model User --request UserInput
berserk make:crud User
berserk make:request CreateUserRequest
berserk make:request UserInput --model User
berserk make:middleware Audit
berserk make:resource UserResource
berserk make:policy UserPolicy
berserk make:migration create_users_table
~~~

The application owns its route registration, configuration, controllers, models, validation, and middleware. You can move generated code into another module structure as long as normal Rust imports and registrations are updated.

## Database migrations

~~~sh
berserk migrate
berserk migrate --dry-run
berserk migrate:status
berserk migrate:rollback
berserk migrate:reset
~~~

See [migrations](migrations.md), [project structure](project-structure.md), and [the public API](public-api.md) for the corresponding Rust interfaces.

## Rust macros

Berserk provides application-facing macros for models and related declarations. They generate ordinary Rust contracts used by the framework and Claw ORM. Refer to [models and presentation](model-presentation.md) and [relationships](relationships.md) for examples and supported attributes.
