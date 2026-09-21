# Models, collections, JSON, and Axe

Enable `claw` for model integration and `view` for Axe rendering. `Model` is
Claw's model contract; ordinary models need no additional serialization or view
trait. No `Clone` bound is placed on the model.

```rust
use berserk::claw::{model_fields, Model, Value};

pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub password_hash: String,
}

impl Model for User {
    const TABLE: &'static str = "users";
    const FILLABLE: &'static [&'static str] = &["name", "email"];
    const HIDDEN: &'static [&'static str] = &["password_hash"];

    model_fields! { id, name, email, password_hash }

    fn key(&self) -> Value { self.id.into() }
    fn parse_route_key(value: &str) -> Option<Value> {
        value.parse::<i64>().ok().map(Into::into)
    }
}
```

The optional, declarative `model_fields!` helper generates `from_row` and
`attributes` from one list. Struct field types infer the decoder. Supported
field values implement `FromValue`, `Clone`, and conversion into the database
`Value`; built-in scalar types and nullable `Option<T>` fields are supported.
Only individual attribute values are cloned. Use `name => "display_name"` for a
renamed column, and use the mapped column name in `HIDDEN`. Manual `Model`
implementations remain valid for custom mappings. Without an `attributes`
override, the compatible safe default exposes only the primary key.

`HIDDEN` applies to the default JSON and view presentation. It does not control
mass assignment (`FILLABLE`), persistence, or Rust's independent `Debug` derive.
No field is hidden by a naming heuristic. Attribute mappings should include
only scalar values deliberately intended for presentation; relationships and
request state are never loaded or exposed implicitly.

## Controllers

Inside a configured request database scope:

```rust
use berserk::{response, view, Response, Result};

pub fn index() -> Result<Response> {
    let users = User::all()?;
    view("users/index", [("users", users)])
}

pub fn show(user: User) -> Result<Response> {
    response().json(user)
}
```

`Response::view(...)` and `response().view(...)` accept the same data. For mixed
types, use Berserk's `view_data!["users" => users, "title" => "Users"]`.
Models and collections may also be borrowed. Berserk's `view_object!` supports
the same conversion in nested objects. Only the explicitly named data enters
the view. The default template location is `app/views` relative to the working
directory.

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

No runtime reflection, procedural macro crate, or new dependency is introduced.
The workspace remains on Rust 1.88.
