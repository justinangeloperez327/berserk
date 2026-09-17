# Phase 17 — Models and relationships

Phase 17 adds typed records above the driver-neutral query builder. The model API is ordinary Rust: derive macros are intentionally deferred until these contracts compile and prove useful.

## Typed models and fields

```rust
use berserk_database::{field, Model, Result, Row, Value};

struct User { id: u64, name: String, active: bool }

impl Model for User {
    const TABLE: &'static str = "users";

    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            name: field(row, "name")?,
            active: field(row, "active")?,
        })
    }

    fn key(&self) -> Value { self.id.into() }
}
```

`field` distinguishes missing columns, NULL, and incompatible database types. `Option<T>` is the only implicit NULL conversion. Integer conversions are checked; text, bytes, and floating-point values are not silently interconverted. Boolean fields also accept database integer booleans `0` and `1` for cross-driver compatibility.

## Model queries and scopes

```rust
fn active(query: Query) -> Query {
    query.where_("active", "=", true)
}

let users = User::query()
    .scope(active)
    .where_("role", "=", "admin")
    .order_by("name", Direction::Asc)
    .get(&mut connection)?;

let user = User::find(&mut connection, 42_u64)?;
```

`ModelQuery` wraps the Phase 16 builder and decodes returned rows into a model. Building, scoping, and inspecting a statement are side-effect free. Only `get`, `first`, `all`, and `find` contact a connection.

## Explicit relationships

Relationships are descriptors, not fields with hidden loading behavior:

```rust
let posts = HasMany::<User, Post>::new(
    "user_id",
    User::key,
    |post| post.user_id.into(),
);

let loaded = posts.load(&mut connection, &users)?;
let first_users_posts = loaded.get(&users[0].key()).unwrap_or(&[]);
```

`HasMany`, `HasOne`, and `BelongsTo` each perform one visible query for a nonempty input set. Duplicate input keys are collapsed, NULL foreign keys are skipped, and an empty input set performs no I/O. Results are returned as `RelatedSet<M>`, grouped by linking key, so no application model is mutated and no model needs to implement `Clone`.

`HasOne` rejects multiple rows for one key instead of silently choosing one. Relationship SQL uses the same bound-value query builder as ordinary queries.

## Deferred scope

Derive macros, polymorphic and many-to-many relationships, pivot models, recursive eager-loading graphs, persistence/change tracking, timestamps, soft deletes, and automatic serialization remain deferred. Lazy loading is not planned as a default because it hides database I/O and enables N+1 query failures.

## Verification status

Test sources cover typed decoding, nullable fields, reusable scopes, primary-key lookup, one-query eager loading, grouping, empty inputs, belongs-to loading, and has-one cardinality errors. Compilation and execution remain pending because Rust tooling is unavailable in this environment.
