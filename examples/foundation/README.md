# Foundation CRUD API

Run from the repository root:

```sh
cargo run -p foundation-example
```

The application listens on 127.0.0.1:3000 and stores users in foundation.sqlite (override with BERSERK_DATABASE). It demonstrates typed FormRequest input, model-bound actions, guarded writes, transactions, unique-email validation plus a database constraint, resources, pagination, named CRUD routes, and an optional async health action.

```sh
curl -H 'content-type: application/json' -d '{"name":" Ada ","email":"ADA@example.com"}' http://127.0.0.1:3000/users
curl 'http://127.0.0.1:3000/users?page=1'
curl http://127.0.0.1:3000/users/1
curl -X PUT -H 'content-type: application/json' -d '{"name":"Ada Lovelace","email":"ada@example.com"}' http://127.0.0.1:3000/users/1
curl -X DELETE http://127.0.0.1:3000/users/1
```

This local teaching example allows unauthenticated CRUD. Add a Guard and authorization policies before exposing private data. The schema setup is intentionally small; production applications should use migrations. No frontend is included. `cargo test -p foundation-example` exercises the real application and SQLite driver.
