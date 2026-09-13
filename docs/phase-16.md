# Phase 16 — Fluent query builder

Phase 16 adds a Laravel-inspired query API while retaining Rust's explicit execution and database differences.

## Public API

```rust
let users = Query::table("users")
    .select(["id", "name"])
    .where_("active", "=", true)
    .where_in("role", ["admin", "editor"])
    .order_by("name", Direction::Asc)
    .limit(25)
    .get(&mut connection)?;
```

Supported composition includes `where_`, `or_where`, `where_in`, `or_where_in`, `where_not_in`, NULL filters, inner/left/right equijoins, multiple order clauses, limit, offset, single-row insert, update, delete, and explicitly raw bound statements.

`to_statement(driver)` is the inspection boundary. It returns SQL plus separately ordered bindings without performing I/O. `get`, `first`, and `execute` are terminal methods and are the only builder methods that contact a connection.

## Dialects

- PostgreSQL uses double-quoted identifiers and numbered `$1` placeholders.
- MySQL uses backtick-quoted identifiers and `?` placeholders.
- SQLite uses double-quoted identifiers and `?` placeholders.
- Offset-without-limit syntax is adapted per backend.

Identifiers accept ASCII letters, digits after the first character, underscores, dotted qualification, and a terminal wildcard. Operators use a fixed allowlist. User values never become SQL text.

## Safety decisions

- Unfiltered update and delete fail unless the developer calls `allow_all()`.
- `get` and `first` reject mutations; `execute` rejects selects.
- NULL comparisons require `where_null` or `where_not_null`.
- Empty IN lists compile to a deterministic false predicate; empty NOT IN lists compile to true.
- Duplicate insert/update columns, invalid identifiers, unsupported operators, and incompatible clauses fail before execution.
- `Query::raw(...)` is deliberately explicit and does not translate placeholders between dialects. Its SQL is caller-owned, while values still use driver binding.

## Deferred scope

Grouped predicates, subqueries, aliases, aggregates, upserts, bulk inserts, returning clauses, row locking, common-table expressions, database-specific extensions, and model-aware queries follow only after the core builder compiles and passes cross-driver tests.

## Verification status

Exact SQL and binding-order test sources cover all three dialects, safety guards, empty lists, injection rejection, inserts, updates, deletes, and raw statements. SQLite also has an end-to-end fluent CRUD test source. Compilation and execution remain pending because no Rust toolchain is available.
