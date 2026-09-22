# Foundation reference application

This is Berserk's end-to-end reference application for the v1.0 maturity gate.
It intentionally combines the framework pieces that are easy to validate only
in isolation: generated-style CRUD contracts, FormRequest validation,
migrations, Claw relationships, bearer authentication, authorization, Axe
views, and in-memory application tests.

## Run

From the repository root:

```sh
BERSERK_SHOW_DEMO_TOKEN=1 cargo run -p foundation-example
```

On PowerShell:

```powershell
$env:BERSERK_SHOW_DEMO_TOKEN="1"
cargo run -p foundation-example
```

The application listens on `127.0.0.1:3000`. The SQLite path defaults to
`foundation.sqlite` and can be overridden with `BERSERK_DATABASE`.

The optional `BERSERK_SHOW_DEMO_TOKEN` flag prints the process-local
development API token. The token is generated with Berserk's
`TokenManager<MemoryTokenStore>`, is stored by digest, and becomes invalid
when the example process restarts. The token is not printed by default.

## Application assembly

Startup runs the registered migration before the application begins serving:

```rust
let mut connection = SqliteConnection::open(&path)?;
migrations::migrate(&mut connection)?;
```

The migration creates:

- `users`
- `posts`
- `profiles`
- `roles`
- `role_user`
- Berserk's migration tracking table

The schema uses the migration DSL, unique constraints, indexes, foreign keys,
and a composite pivot key. The example no longer creates application tables
with ad-hoc startup SQL.

Authentication is registered once on the application:

```rust
app.auth(tokens)?;
```

The CRUD surface uses the same `CrudController` contract emitted by Berserk's
CRUD code generator and is registered atomically:

```rust
api.can("users.manage")?.crud("/users", Users)?;
```

That creates the named `users.index`, `users.store`, `users.show`,
`users.update`, and `users.destroy` routes. The write request also checks
`users.manage` through `FormRequest::authorize_request`, so authorization is
part of the request lifecycle rather than only a controller convention.

## Relationships and views

The user model declares:

- `has_many posts`
- `has_one profile`
- `belongs_to_many roles`

The API index eager-loads all three relationships and paginates the loaded
models. The HTML route eager-loads posts and roles and renders them through Axe.

```rust
let users = User::query()
    .with(["posts", "roles"])
    .order_by("id", Direction::Asc)
    .get()?;
```

`/users/browse` requires `users.read`. The template displays the user,
their posts, and their roles. The build script validates the complete
`app/views` tree through Axe before the example compiles.

## Resource authorization

`GET /users/{id}/policy` uses `Request::authorize` with a typed
`Policy<User>`.

- an `admin` principal can view any user;
- a `user:<id>` principal can view its own matching user;
- the principal must also carry the `users.view` ability.

This demonstrates both route-level ability checks and resource-specific policy
authorization.

## Routes

| Route | Requirement | Purpose |
| --- | --- | --- |
| `GET /health` | public | health smoke route |
| `GET /users` | `users.manage` | paginated users with eager relationships |
| `POST /users` | `users.manage` | validated create |
| `GET /users/{id}` | `users.manage` | route-model-bound show |
| `PUT/PATCH /users/{id}` | `users.manage` | validated update |
| `DELETE /users/{id}` | `users.manage` | delete |
| `GET /users/browse` | `users.read` | Axe relationship view |
| `GET /users/{id}/policy` | authenticated + policy | resource authorization |

## Application tests

The example uses `berserk-testing::TestClient` against the real in-memory
router and middleware stack. Tests verify:

- unauthenticated CRUD returns 401;
- a read-only token cannot write;
- generated CRUD route naming works;
- sanitization normalizes names and email addresses;
- duplicate email validation returns 422;
- create/show/update/delete work through `route.crud`;
- eager `has_many`, `has_one`, and `belongs_to_many` data appears in JSON;
- the Axe route renders related posts and roles;
- resource policy ownership allows and denies the expected identities;
- the actual migration runner creates the test schema.

Repository CI builds and tests this package as a workspace member, and the
foundation build script validates the real Axe view tree.
