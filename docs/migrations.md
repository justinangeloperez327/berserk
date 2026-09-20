# Migrations

Berserk 1.1 provides a driver-neutral migration DSL in `berserk::database::migrations`. Migration definitions are compiled for PostgreSQL, MySQL, or SQLite and executed by the existing `MigrationRunner`.

## Create a table

```rust
use berserk::database::{
    migrations::{Column, ForeignAction, ForeignKey, Index, MigrationPlan, Table},
    Driver, Migration, Result, Statement,
};

pub struct CreatePostsTable;

impl Migration for CreatePostsTable {
    fn name(&self) -> &'static str {
        "202609200001_create_posts_table"
    }

    fn up(&self, driver: Driver) -> Result<Vec<Statement>> {
        MigrationPlan::new()
            .create(
                Table::create("posts")
                    .columns([
                        Column::id(),
                        Column::big_integer("user_id"),
                        Column::string("title"),
                        Column::text("body").nullable(),
                        Column::timestamp("created_at").default_current_timestamp(),
                        Column::timestamp("updated_at").default_current_timestamp(),
                    ])
                    .indexes([Index::new(["user_id"])])
                    .foreign_keys([
                        ForeignKey::new(["user_id"])
                            .references("users", ["id"])
                            .on_delete(ForeignAction::Cascade),
                    ]),
            )
            .compile(driver)
    }

    fn down(&self, driver: Driver) -> Result<Vec<Statement>> {
        MigrationPlan::new()
            .table(Table::drop("posts"))
            .compile(driver)
    }
}
```

Timestamps are ordinary explicit columns. Berserk does not inject `created_at` or `updated_at` automatically.

## Alter a table

```rust
MigrationPlan::new()
    .alter(
        Table::alter("users")
            .add_columns([Column::string("phone").nullable()])
            .rename("phone", "mobile")
            .drop_columns(["legacy_column"]),
    )
    .compile(driver)
```

Direct column modification is compiled for PostgreSQL and MySQL. SQLite direct modification is rejected because SQLite requires table-rebuild semantics for many column changes; write an explicit rebuild migration instead of relying on lossy emulation.

## Table operations

```rust
Table::rename("users", "accounts");
Table::drop("accounts");
Table::drop_if_exists("accounts");
```

## Keys and indexes

Use `Column::id()` for the conventional generated numeric primary key. Explicit and composite primary keys are supported with `.primary([...])`.

```rust
Table::create("role_user")
    .columns([
        Column::big_integer("role_id"),
        Column::big_integer("user_id"),
    ])
    .primary(["role_id", "user_id"])
    .indexes([
        Index::new(["user_id"]),
        Index::new(["role_id", "user_id"]).unique(),
    ]);
```

Foreign keys are table constraints rather than column types. This keeps UUID, integer, and composite foreign keys representable without inventing special foreign-key column types.

## Supported column types

The portable DSL includes identifiers, strings, text, tiny/small/regular/big integers, decimal, float, double, boolean, date, time, datetime, timestamp, JSON, binary, and UUID columns. Database-specific compilers map these definitions to the closest supported native representation.

## Safety and portability

Migration identifiers are validated before SQL generation and limited to a portable 63-character ASCII identifier surface. String defaults are escaped by the compiler. Unsupported operations return errors rather than silently changing semantics.

The migration runner records applied migrations in `__framework_migrations`, assigns batches, applies pending migrations in registration order, rolls the latest batch back in reverse registration order, and can roll back all registered batches.
