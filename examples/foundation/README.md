# Foundation CRUD API

Run from the repository root:

```sh
cargo run -p foundation-example
```

The application listens on 127.0.0.1:3000 and stores users in foundation.sqlite (override with BERSERK_DATABASE). Its synchronous actions ask only for what they need: `index()`, `show(id: u64)`, `store(Request)`, `update(id: u64, Request)`, and `destroy(id: u64)`.

Store and update call `request.validate::<UserInput>()`; input is sanitized before semantic validation and request-scoped uniqueness checks. Claw uses the existing request database scope, guarded writes, a transaction for creation, and a database UNIQUE constraint. The controller explicitly maps public fields through `ApiResource` and returns `response().status(201).json(user)` on creation. Show/update return a plain user object; list keeps the existing paginated `data`/`meta` envelope; delete returns 204. Invalid primitive IDs return 400 and missing users return 404.

The routes use individual verbs to demonstrate primitive-ID and manual-Request actions. For model-bound actions and automatically extracted FormRequests, the existing `route.crud(...)`/`CrudController` contract remains the canonical shortcut. No additional resource alias is introduced.

Global RequestId and HandleErrors layers wrap the existing route-group middleware, which adds `x-api-version: 0.3` to successful CRUD responses. `/health` is a synchronous text action; the example no longer requires the optional `async` or `auth` features.

```sh
curl -H 'content-type: application/json' -d '{"name":" Ada ","email":"ADA@example.com"}' http://127.0.0.1:3000/users
curl 'http://127.0.0.1:3000/users?page=1'
curl http://127.0.0.1:3000/users/1
curl -X PUT -H 'content-type: application/json' -d '{"name":"Ada Lovelace","email":"ada@example.com"}' http://127.0.0.1:3000/users/1
curl -X DELETE http://127.0.0.1:3000/users/1
```

This local teaching example allows unauthenticated CRUD. Add a Guard and authorization policies before exposing private data. The schema setup is intentionally small; production applications should use migrations. PUT and PATCH both use the same complete name/email input contract; partial updates are not implemented. No frontend is included.

This implementation pass did not compile or execute the example. Manual follow-up: run `cargo test -p foundation-example`, exercise the requests above, and check malformed/overflowing IDs, duplicate email rejection, sanitation, update/delete behavior, pagination, and middleware headers.
