# Phase 18 — Database tooling

Phase 18 adds migrations, seeders, factories, and pagination above the shared database contracts. The tools remain driver-neutral and explicitly execute through a caller-provided connection.

## Migrations

```rust
struct CreateUsers;

impl Migration for CreateUsers {
    fn name(&self) -> &'static str { "202609120001_create_users" }

    fn up(&self, driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![Statement::new(sql_for_create_users(driver))])
    }

    fn down(&self, driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![Statement::new(sql_for_drop_users(driver))])
    }
}

let runner = MigrationRunner::new([&CreateUsers as &dyn Migration])?;
let report = runner.migrate(&mut connection)?;
```

The runner creates `__framework_migrations`, validates unique nonempty migration names, assigns monotonically increasing batches, skips applied migrations, and records a migration only after every `up` statement succeeds. `rollback_last` processes the newest batch in reverse recorded order and fails if an applied migration is no longer registered.

Migration SQL is deliberately supplied by the migration. This keeps backend differences visible instead of pretending every schema operation is portable. The tracking statement uses bound values.

DDL transaction behavior differs between PostgreSQL, MySQL, and SQLite. The runner therefore does not promise cross-driver atomic DDL. A failed multi-statement migration can require manual cleanup, and migrations should be small and restart-safe.

## Seeders and factories

```rust
run_seeders(&mut connection, &[&RolesSeeder, &AdminSeeder])?;
let users = UserFactory.create_many(&mut connection, 10)?;
```

Seeders run in the exact supplied order and stop at the first error. They decide whether repeated execution is safe.

A factory implements deterministic `make(index)` and application-specific `persist`. `make_many` performs no I/O; `create_many` visibly persists each generated value. Random-data dependencies are not required by the database crate.

## Pagination

```rust
let users = User::query()
    .where_("active", "=", true)
    .order_by("name", Direction::Asc)
    .paginate(&mut connection, 2, 25)?;
```

Pagination executes a filtered `COUNT(*)` query followed by one `LIMIT`/`OFFSET` data query. `Page<T>` exposes items, current page, page size, total records, last page, and previous/next indicators. Pages begin at 1; page size must be between 1 and 1000; offset overflow is rejected.

Count queries retain filters and joins but intentionally ignore selected columns, ordering, limit, and offset. Joined queries can count joined rows rather than distinct models; callers needing distinct or grouped totals should use an explicit query until aggregate support is expanded.

## Deferred scope

Schema builders, migration file discovery, CLI generation/commands, advisory migration locks, checksums, transactional policies, batch inserts, factory states, fake-data providers, cursor pagination, distinct/grouped counts, and resumable data migrations remain deferred.

## Verification status

Test sources cover migration preparation, tracking order, rollback, pagination metadata and SQL shape, ordered seeders, and deterministic factories. Manifest paths, file integrity, and archive integrity are checked locally. Rust compilation and execution remain pending because the toolchain is unavailable.
