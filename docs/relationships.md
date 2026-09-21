# Claw ORM relationships — Berserk 1.0

Relationships are explicit descriptors over existing `Model`, `ModelQuery`, and database contracts. They never perform hidden lazy loading and require no `Clone` bound on models. This guide uses `User`, `Post`, `Profile`, `Role`, and `role_user`; the [runnable example](../crates/claw/examples/relationships.rs) defines the complete models and schema.

```sh
cargo run -p claw-orm --example relationships --features sqlite
```

## Declare relationships

With Berserk's `Model` derive, relationships are declared on the model and
compile into the same typed Claw descriptors used by the lower-level API:

```rust
use berserk::Model;

#[derive(Model)]
#[table("users")]
#[has_many(Post, "posts", foreign_key = "user_id")]
#[has_one(Profile, "profile", foreign_key = "user_id")]
#[belongs_to_many(
    Role,
    "roles",
    pivot = "role_user",
    foreign_pivot_key = "user_id",
    related_pivot_key = "role_id"
)]
pub struct User {
    #[primary_key]
    pub id: i64,

    #[fillable]
    pub name: String,
}

#[derive(Model)]
#[table("posts")]
#[belongs_to(User, "user", foreign_key = "user_id")]
pub struct Post {
    #[primary_key]
    pub id: i64,

    #[fillable]
    pub user_id: i64,

    #[fillable]
    pub title: String,
}
```

The derive generates `User::posts()`, `User::profile()`, `User::roles()`,
and `Post::user()`. Relationship declarations create descriptors only: they
do not load related rows, add relationship fields to the struct, or perform SQL.

| Attribute | Generated descriptor |
| --- | --- |
| `#[has_many(Post, "posts", foreign_key = "user_id")]` | `HasMany<User, Post>` |
| `#[has_one(Profile, "profile", foreign_key = "user_id")]` | `HasOne<User, Profile>` |
| `#[belongs_to(User, "user", foreign_key = "user_id")]` | `BelongsTo<Post, User>` |
| `#[belongs_to_many(Role, "roles", ...)]` | `BelongsToMany<User, Role>` |

The relationship name is the generated Rust method name. `foreign_key` must
refer to a mapped model column. Missing mapped keys are reported as `Decode`
errors when a relationship is evaluated instead of silently producing a NULL
grouping key. Many-to-many declarations require the pivot table and both pivot
key column names. Parent and related primary keys come from each model's
`#[primary_key]` metadata.

Manual relationship methods remain supported for custom key mappings or for
applications using `claw-orm` directly:

```rust
impl User {
    fn posts() -> HasMany<Self, Post> {
        HasMany::new("user_id", Self::key, |post: &Post| post.user_id.into())
    }
}
```

The existing constructors remain source-compatible. Derive-generated
relationships use fallible constructor variants internally so metadata lookup
errors stay typed. There is still no runtime field reflection and no hidden
lazy loading.

Use `NOT NULL` and foreign keys on the pivot, `UNIQUE(user_id, role_id)` for
unique links, and `UNIQUE(profiles.user_id)` for one profile per user. Index
relationship foreign keys for large tables. The ORM does not silently create
constraints.

## Query one parent's relationships

All four descriptors offer `query_for(&parent) -> Result<ModelQuery<Related>>`. Construction validates the key and performs no SQL; normal terminal methods execute the query.

```rust
let posts = User::posts()
    .query_for(&user)?
    .where_("published", true)
    .order_by("id", Direction::Desc)
    .get()?;

let profile = User::profile().query_for(&user)?.first()?;
let owner = Post::user().query_for(&post)?.first()?;
let roles = User::roles()
    .query_for(&user)?
    .where_("roles.name", "Editor")
    .order_by("roles.id", Direction::Asc)
    .get_on(&mut connection)?;
```

These are ordinary `ModelQuery` values: `select`, filters, ordering, `count`, `exists`, `first`, `get`, `paginate`, and eager loading remain available. Many-to-many queries join the pivot to the related table and select only related-table columns. Qualify ambiguous columns, such as `roles.id`, when both tables have an `id` column. Joined updates and deletes are rejected by the existing portable database builder; use pivot operations to change links, or a separate model query to change roles themselves.

Parent restrictions use the additive `Query::constrain_in` primitive. SQL is equivalent to `parent restriction AND (ordinary filters)`, so `or_where` cannot accidentally include another parent's rows. An absent belongs-to key matches no records even with an OR filter. Deliberately replacing the builder inside `scope` starts a different query; this is not an authorization boundary.

`HasOne::query_for` is an ordinary query and can inspect multiple rows. `first` has normal first-row semantics; it is not a cardinality assertion. `HasOne::load_on` and eager loading enforce zero or one record per parent and fail on duplicates.

## Eager loading and results

For application code, the preferred eager-loading syntax is relationship names:

```rust
let users = User::query()
    .with(["posts", "profile", "roles"])
    .get()?;
```

A single relationship is also accepted:

```rust
let users = User::query()
    .with("posts")
    .get()?;
```

An empty list is valid and performs no relationship queries:

```rust
let users = User::query()
    .with([])
    .get()?;
```

The names are validated against the relationships declared by `#[derive(Model)]`.
Unknown names return `InvalidInput`. Duplicate names are loaded once. Each
requested relationship is batch-loaded across the complete parent result, so the
number of queries does not grow per parent.

Named eager loading returns `Loaded<M, NamedRelations>`. Berserk's JSON and Axe
presentation adapters include requested relations directly in each parent object:
to-many relations become arrays; to-one relations become an object or `null`.
Only each related model's visible attributes are exposed, so `#[hidden]` still
applies.

```rust
let users = User::query()
    .with(["posts", "roles"])
    .get()?;

response().json(users)?;
```

The lower-level typed eager API remains available when application code needs the
actual related Rust model types rather than presentation-safe named data:

```rust
let loaded = User::query()
    .order_by("id", Direction::Asc)
    .with(User::posts())
    .with(User::roles())
    .get_on(&mut connection)?;

let (posts, roles) = loaded.relations;
for user in &loaded.models {
    let user_posts = posts.get(&user.key()).unwrap_or(&[]);
    let user_roles = roles.get(&user.key()).unwrap_or(&[]);
}
```

| Public type | Meaning |
| --- | --- |
| `Relationship<M>` | Typed batch-loading contract |
| `RelatedSet<R>` | Typed related models grouped by linking key |
| `EagerQuery<M, R>` | Zero-erasure typed eager query |
| `NamedEagerQuery<M>` | Relationship-name eager query |
| `NamedRelations` | Requested named relationship output |
| `Loaded<M, R>` | Parent collection plus eager relationship output |
| `LoadedPage<M, R>` | Parent page plus eager relationship output |

Named loading is explicit eager loading, not lazy loading. Accessing a relationship
that was not requested does not execute another query. Flat declared relationship
names are supported by this API; dotted nested relationship paths are not resolved
implicitly.

### Query counts and pagination

```rust
let loaded = User::query()
    .order_by("id", Direction::Asc)
    .with(User::roles())
    .paginate_on(&mut connection, 2, 20)?;

// Only these page items have relationships loaded.
let users = loaded.page.items();
let total_users = loaded.page.total();
```

Scoped `paginate(20)` uses the active page context, defaulting to page 1 outside a request page context. Explicit `paginate_on(connection, page, per_page)` selects the page directly. Existing page validation applies: positive page and size, size at most 1,000, checked offset arithmetic.

| Load shape, with matching parents and links | SQL queries |
| --- | --- |
| Users only | 1 |
| Users + posts or profiles | 2 |
| Users + roles | 3: users, pivot, roles |
| Users + posts + roles | 4 |
| Paginated users + roles | 4: count, users page, pivot, roles |

Empty parent results skip relationship queries. Empty pivot results skip the related-model query. Query counts do not grow per parent, avoiding N+1 behavior. Calling `query_for(...).get()` separately inside a parent loop still performs one query per iteration; request eager loading instead. Loading is batched, not unbounded streaming: paginate parent queries to respect driver parameter limits and memory budgets. Eager loading does not itself start a transaction or promise a consistent snapshot across concurrent writes; use the existing transaction scope if required.

## Attach, detach, and sync

```rust
let roles = User::roles();
roles.attach(&user, 10)?;
roles.attach_many(&user, [11, 12])?;
roles.detach(&user, 12)?;
roles.detach_many(&user, [10, 11])?;
roles.detach_all(&user)?;

let changes = roles.sync(&user, [10, 11, 11])?;
// changes.attached and changes.detached are distinct Vec<Value> keys.
```

Every mutation has an explicit connection equivalent with the connection first:

```rust
roles.attach_on(&mut connection, &user, 10)?;
roles.attach_many_on(&mut connection, &user, [11, 12])?;
roles.detach_on(&mut connection, &user, 10)?;
roles.detach_many_on(&mut connection, &user, [11, 12])?;
roles.detach_all_on(&mut connection, &user)?;
let changes = roles.sync_on(&mut connection, &user, [10, 12])?;
```

| Operation | Result and behavior |
| --- | --- |
| `attach` | `Execution`; inserts one row without a pre-read or conflict suppression |
| `attach_many` | `u64` affected rows; validates/deduplicates input, inserts atomically |
| `detach` / `detach_many` | `Execution`; deletes matching links only for this parent |
| `detach_all` | `Execution`; explicitly deletes all links for this parent |
| `sync` | `SyncResult { attached, detached }`; atomically reconciles key membership |

Empty attach/detach collections are no-ops. **Empty sync removes all links for that parent.** An empty Rust array needs a key type when inference has no other source: `roles.sync(&user, [] as [u64; 0])?`.

Mutation and query keys support signed/unsigned integers and nonempty text/bytes. Null, boolean, floating-point, and empty text/byte keys are rejected with `InvalidInput`; zero and negative integers are valid. The database still controls column types, range, collation, and referential constraints. Values are bound parameters; table and column identifiers go through the existing identifier validator.

### Atomicity and concurrency

`sync` validates requested keys before starting work, then uses the existing transaction infrastructure to read current IDs, calculate membership differences, delete removed links, insert new links, and commit. Any intermediate failure rolls back. Commit failures are propagated. If rollback itself fails, the existing transaction layer reports the rollback error and cannot promise recovery on a failed connection.

`attach_many` also owns one transaction. These operations reject nesting before changing rows because Berserk's current transaction abstraction does not expose nested transactions or savepoints. Single `attach`, `detach`, and `detach_all` can participate in an outer `Transaction::run` scope. Do not split `sync` into manual nontransactional steps to bypass this rule.

Atomicity means all-or-nothing changes; it does not serialize concurrent synchronizations for the same parent. Use a UNIQUE constraint and application/database coordination for competing writers. Key comparison uses `Value` equality plus signed/unsigned integer normalization; application keys should use a canonical representation when database collation considers different text values equal.

### Duplicate and missing-record semantics

- Loading and joined relationship queries preserve actual duplicate pivot rows. Shared related rows are decoded separately into each parent's group without requiring `R: Clone`.
- `attach` inserts again when uniqueness is unconstrained. A database UNIQUE violation is returned unchanged when a constraint exists.
- `attach_many` and `sync` deduplicate requested keys, including equivalent signed/unsigned integers. They never silently suppress database errors.
- `sync` preserves duplicate rows for retained keys and deletes every row for removed keys. Its result reports changed keys, not deleted-row counts. Nonnegative integer result keys use `Value::U64`; attached keys follow input order and detached order is database-dependent.
- `detach` removes every matching duplicate link for one parent. Other parents sharing the same related key are unaffected.
- Dangling links whose related records are missing produce no related model, matching existing loading behavior and the joined query. Use database foreign keys to prevent these links.
- Missing pivot columns and NULL pivot keys encountered by batch loaders produce `Decode`. Null parent keys are skipped by existing batch loaders. A joined SQL query naturally excludes NULL or dangling joins; it does not inspect malformed pivot rows as a separate step.

## Errors and compatibility

Claw continues using `DatabaseError`, `ErrorKind`, and the existing `Result`. Invalid mutation/query keys report `InvalidInput`; malformed rows, model decoding failures, and violated HasOne cardinality report `Decode`. Invalid SQL identifiers and unsupported joined mutations report `Query`. Driver query, constraint, connection, and transaction errors propagate. A scoped terminal operation without a connection scope reports `Configuration`. No new error hierarchy is introduced.

The 1.0 inherent signatures `HasMany::load(connection, parents)`, `HasOne::load(connection, parents)`, and `BelongsTo::load(connection, children)` are restored as deprecated forwarding methods after their removal during pre-release development. Existing callers continue compiling. Use `load_on(connection, models)` for explicit loading. For scoped direct loading of any relationship, use:

```rust
let profiles = Relationship::load(&User::profile(), &users)?;
let roles = Relationship::load(&User::roles(), &users)?;
```

Rust cannot overload an inherent method by argument count, so scoped loading cannot reuse `.load(models)` on the three legacy types until a major release removes their deprecated signatures. `BelongsToMany`, added during pre-release development, supports `.load(models)` directly. Ordinary `get/get_on`, `find/find_on`, `save/save_on`, `update/update_on`, and `paginate/paginate_on` are unchanged.

HasOne's public representation and all constructor signatures remain intact. There are no pivot models, nested eager traversal, polymorphic relationships, soft deletes, observers, automatic timestamps, or dynamic properties in the 1.0 target. Relationship attributes are compile-time declarations that generate the existing typed descriptors.
