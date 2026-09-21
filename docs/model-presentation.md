# Models, collections, JSON, and Axe

Enable `claw` for model integration and `view` for Axe rendering. `Model` is
Claw's model contract; ordinary models need no additional serialization or view
trait. No `Clone` bound is placed on the model.

```rust
use berserk::Model;

#[derive(Model)]
#[table("users")]
pub struct User {
    #[primary_key]
    pub id: i64,

    #[fillable]
    pub name: String,

    #[fillable]
    pub email: String,

    #[fillable]
    #[hidden]
    pub password: String,
}
```

`#[derive(Model)]` generates the Claw `Model` implementation, including row
hydration, attribute mapping, primary-key handling, route-key parsing,
`FILLABLE`, and `HIDDEN`. `#[table("...")]` is required, exactly one field
must be marked `#[primary_key]`, and fields opt into mass assignment with
`#[fillable]`. `#[hidden]` removes a mapped field from default JSON and Axe
presentation without removing it from hydration or persistence. Use
`#[column("database_name")]` when a Rust field maps to a different database
column.

Relationships can be declared on the model without becoming row fields:

```rust
#[derive(Model)]
#[table("users")]
#[has_many(Post, "posts", foreign_key = "user_id")]
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
}
```

This generates typed descriptor methods such as `User::posts()` and
`User::roles()`. Declaring a relationship does not lazy-load or serialize it
automatically; querying and eager loading remain explicit Claw operations.

For Laravel-style eager loading, use the declared relationship names:

```rust
let users = User::query()
    .with(["posts", "roles"])
    .get()?;
```

The returned loaded collection keeps the parent models and named relation data
together. Berserk's JSON and Axe adapters render the requested relations inside
each parent object. `User::query().with([]).get()?` is valid and loads no
relationships. Typed eager loading with `with(User::posts())` remains available
when code needs concrete related Rust model values.

Struct field types still determine decoding and database-value conversion.
Mapped values therefore need the same capabilities as manual models: decoding
through Claw, cloning for presentation, and conversion into `Value`. The
existing `model_fields!` helper and handwritten `impl Model` remain supported
for custom mappings or lower-level Claw usage.

`#[hidden]` does not control mass assignment, persistence, or Rust's independent
`Debug` implementation. No field is hidden by a naming heuristic.
Relationships and request state are never loaded or exposed implicitly.

## Controllers

Inside a configured request database scope:

```rust
use berserk::{response, view, Response, Result};

pub fn index() -> Result<Response> {
    let users = User::all()?;
    view("users/index").with([("users", users)])
}

pub fn show(user: User) -> Result<Response> {
    response().json(user)
}
```

The free helper is fluent: `view("users/index").with(data)`. Use
`view("users/index").render()` for a template that needs no application data.
`Response::view(...)` and `response().view(...)` retain their direct
`(view, data)` form. For mixed types, use Berserk's
`view_data!["users" => users, "title" => "Users"]`. Models and collections may
also be borrowed. Berserk's `view_object!` supports the same conversion in
nested objects. Only the explicitly named data enters the view. The default
template location is `app/views` relative to the working directory.

```html
<h1>Users</h1>
@foreach(user in users)
    <article>
        <h2>{{ user.name }}</h2>
        <p>{{ user.email }}</p>
    </article>
@endforeach
```

Normal interpolation escapes HTML. Accessing a hidden field is a missing-field
render error; its value is not present in Axe's context. Axe itself has no Claw
dependency and continues to accept its native values and contexts.

## Collections and resources

`all`, `find_many`, and query `get` return `Collection<T>`, including their
explicit-connection counterparts. Eager-loaded parent sets use it too. Slice
access, borrowed/mutable/owned iteration, Rust iterator adapters, `map`, and
`filter` remain available.

`Page<T>` owns a `Collection<T>`. `items()` still returns a slice and
`into_items()` still returns a vector. `collection()` and `into_collection()`
expose the collection directly. `map` changes an intentional item representation
while preserving pagination metadata.

```rust
response().json(User::all()?)?;       // JSON array
response().resource(user)?;           // { "data": { ... } }
response().collection(User::all()?)?; // { "data": [ ... ] }
ResourceCollection::page(User::query().paginate(20)?).into_response()?;
```

`ResourceCollection::new` and the response collection helper accept an
`IntoIterator`, including `Collection<T>`, references, arrays, and vectors. The
resource collection keeps its existing `into_inner() -> Vec<T>` API. JSON
preserves signed/unsigned integers, booleans, strings, nulls, and finite floating
point values. Bytes become arrays of integers. Non-finite floats return an
error rather than invalid JSON.

Explicit `ApiResource` implementations remain available for intentionally
different API output, usually on a separate `PublicUserResource(User)` type.
`Resource<T>` keeps its explicit-resource contract, constructor, and
`into_inner`. Use `response().resource(model)` for the default model envelope.
For a custom paginated representation, use
`ResourceCollection::page(page.map(PublicUserResource))`.

## Compatibility decisions

- The existing query API, model binding, scoped/custom binding, input contracts,
  and specialized `PersistableModel` remain intact. Presentation never writes
  attributes back to the database.
- A legacy type implementing both `Model` and `ApiResource` has two intentional
  representations. Use `response().json(Resource::new(value))` to select its
  custom resource mapping, or remove the redundant `ApiResource` implementation
  to select model attributes. For a page, use `page.map(Resource::new)` to keep
  the custom resource representation. Berserk does not silently pick one.
- Use `berserk::view_data!`/`view_object!` for model data. The standalone Axe
  macros, `Context::insert`, and `From` conversions remain native-value APIs.
  Direct `Collection<Model> -> axe::Value` conversion was moved out of Axe.
  Axe's former `claw` feature name remains accepted as a no-op.
- The response/view helpers infer internal adapter type parameters. Explicit
  turbofish callers may need an additional inferred `_`. Resource collection
  constructors now require a supported presentation type when constructed.

The derive is compile-time only; Berserk still uses no runtime reflection. Manual Claw models remain available, and the workspace remains on Rust 1.88.
