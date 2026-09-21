# Foundation users API and Axe view

Run from the example directory so named Axe views resolve under `app/views`:

```sh
cd examples/foundation
cargo run -p foundation-example
```

The application listens on 127.0.0.1:3000 and stores users in foundation.sqlite (override with BERSERK_DATABASE). Its synchronous actions ask only for what they need: `index()`, `show(User)`, `store(UserInput)`, `update(User, UserInput)`, and `destroy(User)`.

Store and update receive `UserInput` through direct `FormRequest` extraction. Input is decoded, sanitized, authorized, semantically validated, and then checked with request context before the action runs. Show, update, and destroy receive `User` through Claw route-model binding. Claw uses the existing request database scope, guarded writes, a transaction for creation, and a database UNIQUE constraint. The model uses `model_fields! { id, name, email }` once for hydration and presentation. Controllers return its visible attributes automatically, including `response().status(201).json(user)` on creation. Show/update return a plain user object; list keeps the existing paginated `data`/`meta` envelope; delete returns 204. Invalid or missing route-model keys resolve through the model-binding boundary; missing users return 404.

The routes use individual verbs while the handlers demonstrate the same typed plumbing used by Berserk's CRUD contracts: route-model binding for `User` and direct `FormRequest` extraction for `UserInput`. The existing `route.crud(...)`/`CrudController` contract remains the canonical shortcut when an application wants conventional CRUD registration.

`Users::browse` is the HTML counterpart to the paginated API:

```rust
pub fn browse() -> Result<Response> {
    let users = User::all()?;
    view("users/index", [("users", users)])
}
```

The template loops over `users` and displays each name and email. Only explicitly passed data enters the view. If the model gains sensitive attributes, list their mapped names in `Model::HIDDEN` to exclude them from both default JSON and Axe; `FILLABLE` controls writes separately. A separate `ApiResource` remains available for an intentional API representation.

Global RequestId and HandleErrors layers wrap the existing route-group middleware, which adds `x-api-version: 0.3` to successful CRUD responses. `/health` is a synchronous text action; the example no longer requires the optional `async` or `auth` features.

```sh
curl -H 'content-type: application/json' -d '{"name":" Ada ","email":"ADA@example.com"}' http://127.0.0.1:3000/users
curl 'http://127.0.0.1:3000/users?page=1'
curl http://127.0.0.1:3000/users/1
curl -X PUT -H 'content-type: application/json' -d '{"name":"Ada Lovelace","email":"ada@example.com"}' http://127.0.0.1:3000/users/1
curl -X DELETE http://127.0.0.1:3000/users/1
```

This local teaching example allows unauthenticated CRUD. Add a Guard and authorization policies before exposing private data. The schema setup is intentionally small; production applications should use migrations. PUT and PATCH both use the same complete name/email input contract; partial updates are not implemented. The `/users/browse` page demonstrates an Axe view with the same models.

Repository CI compiles and tests the foundation example as part of the workspace. Manual HTTP smoke testing can additionally exercise duplicate email rejection, sanitization, update/delete behavior, pagination, and middleware headers.
