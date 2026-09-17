# Public API contract

Status: current pre-release contract for the `0.1.0` candidate. The API is implemented and tested, but it is not yet stable and may change before the first stable release.

## Application

- `App::new() -> App`: create an application with default server configuration.
- `App::with_config(ServerConfig) -> Result<App>`: validate and use custom server limits.
- `app.route() -> Route<'_>`: borrow the instance route registrar.
- `app.listen(address) -> Result<()>`: bind and run the server.
- `app.bind(address) -> Result<Server>`: bind without immediately entering the run loop.
- `app.path_for(name, params) -> Result<String>`: reverse a named route and percent-encode supplied path-segment values.
- Feature-gated subsystems such as database access are registered explicitly on the application rather than discovered from folders.

Berserk keeps application assembly instance-based. No global router, controller registry, folder convention, or macro is required.

## Routing

Routes are registered through `app.route()` using `get`, `post`, `put`, `patch`, `delete`, `head`, and `options`.

The registrar also supports:

- `prefix(...)` for scoped path prefixes;
- `middleware(...)` for scoped middleware;
- `group(...)` for atomic grouped registration;
- `name(...)` for named routes;
- `resource(...)` and `api_resource(...)` for conventional REST resources;
- one root-scoped fallback.

Routing behavior:

- Parameters use `{name}` and capture exactly one nonempty segment.
- Static segments take precedence over parameter segments from left to right.
- Registration order does not decide path specificity.
- `/users` and `/users/` are distinct.
- Query strings do not participate in matching.
- Equivalent parameter patterns for the same method are rejected even when parameter names differ.
- Missing paths return 404.
- A matching path without a matching method returns 405 with `Allow`.
- Explicit `HEAD` takes priority; otherwise `GET` supplies the representation and the body is suppressed.
- Route groups and generated resource routes register atomically: a conflict prevents the whole staged group from being committed.
- Reverse routing rejects unknown/missing parameters and encodes supplied values as path segments.

## Handlers and controllers

Handlers are ordinary functions or closures. The signature declares what Berserk should extract.

Supported handler inputs include:

- no arguments;
- an owned `Request`;
- one or two typed route parameters in route-template order;
- `Validated<T>` request input;
- combinations of typed route parameters, validated input, and `Request`;
- with the `claw` feature, route-bound models and scoped nested route models.

A typed route parameter that cannot be parsed returns 400. A route model with no matching row returns 404. Scoped nested route-model binding performs the child lookup through `ScopedRouteModel<Parent>`, so a child outside the parent scope also resolves as 404.

Controller resource contracts are represented by `ApiResourceController` and `ResourceController`. `ActionResult` is the conventional `Result<Response>` alias for controller actions that can fail.

## Request and input

Core request access includes method, raw path, path parameters, query string, headers, buffered body, and UTF-8 text conversion. Request fields remain private.

Higher-level input APIs include:

- `Request::json()` for `application/json` bodies;
- `Request::query()` for form-style query decoding, preserving repeated keys and decoding `+`/percent escapes;
- `Request::multipart(max_parts, max_part_headers)` for the bounded buffered multipart subset;
- handler-level `Validated<T>` extraction.

Validated controller input follows one fixed order:

1. decode JSON into the application type;
2. call `sanitize()`;
3. call `validate()`;
4. deliver `Validated<T>` only after validation succeeds.

Malformed JSON/query input returns a controlled 400-class response. Validation failures return 422 with structured field errors. Multipart parsing does not write files or trust client filenames; applications decide how accepted bytes are stored.

## Headers and responses

Framework header names use HTTP token syntax. Header values deliberately use a strict text subset: HTAB plus printable ASCII (`0x20..=0x7e`). CR, LF, NUL, other control bytes, DEL, and obs-text/non-ASCII bytes are rejected.

Responses provide:

- `Response::text(...)`;
- `Response::bytes(...)`;
- `Response::empty()`;
- `Response::json(...)`;
- `.status(...)`;
- validated header insertion/appending.

HTTP framing metadata remains transport-owned where required. Invalid response metadata is not emitted verbatim to the peer, and public failures avoid leaking internal diagnostic details.

## Middleware and operational APIs

Middleware composes through `Middleware` and `Next`. The framework also exposes optional/common operational helpers for request IDs, authentication extraction, request logging, tracing, metrics, health checks, and rate limiting.

These components are explicit layers. Applications remain responsible for choosing which layers protect which routes and for selecting deployment-specific policies such as proxy trust, authentication requirements, and rate limits.

## Server, concurrency, and shutdown

The current server is implemented on bounded Tokio/Hyper infrastructure while preserving synchronous public controller handlers.

`ServerConfig` bounds server resources including worker/queue behavior, request metadata/body sizes, and network deadlines. Deliberate overload is rejected rather than silently corrupting accepted work.

`server.shutdown_handle()` returns a cloneable shutdown handle. Shutdown stops new acceptance and drains accepted work before the server returns. Network deadlines bound I/O waits.

Rust cannot safely force-stop arbitrary synchronous application code. A handler that blocks forever can therefore prevent graceful draining from completing; applications must keep request-path work bounded.

The built-in server does not terminate TLS. Internet-facing deployments require trusted HTTPS termination or a vetted TLS adapter.

## Database and Claw

Database and Claw APIs are feature-gated.

- `request.connection()` lazily acquires and reuses a request-scoped database connection across non-overlapping borrows.
- `request.transaction(...)` reuses that connection, commits on success, rolls back on application error, and rejects unsupported nested request transactions.
- Query-builder values remain bound separately from generated SQL text.
- `Statement` keeps SQL and bindings separate; its default `Debug` output redacts both SQL text and binding values.
- Raw SQL remains an explicit escape hatch and must not interpolate untrusted input.

Claw provides `Model`, optional `PersistableModel`, route-model binding, query builders, and terminal operations. Query chains such as `where_`, `or_where`, `where_in`, `where_not_null`, `order_by`, and `limit` build operations; terminal methods such as `get`, `first`, `count`, `exists`, `update`, `delete`, and `paginate` perform database I/O.

Model lifecycle helpers include create/update/delete, `save()` for models opting into persistence, `fresh()`, and `refresh()`. Primary keys are used as write filters and are not silently accepted as ordinary persisted fields by `save()`.

## Authentication

The `auth` feature provides password, bearer-session, principal, guard, and authorization primitives.

- Passwords are hashed/verified through Argon2 password-hashing APIs.
- `Secret` and `SessionToken` redact their default `Debug` output.
- Session tokens use OS randomness and are stored by digest rather than plaintext token in the provided store contract.
- Session records expire and can be revoked/pruned.

The built-in primitives do not automatically define browser-cookie or CSRF policy. Applications using cookies must define Secure, HttpOnly, SameSite, rotation/revocation, and CSRF behavior appropriate to their deployment.

`MemorySessionStore` is process-local and unbounded; production systems exposed to untrusted session creation should use a bounded/persistent store implementation.

## Storage

`StoragePath` accepts normalized relative paths and rejects absolute paths, empty/`.`/`..` components, backslashes, NUL, and control bytes.

`LocalStorage` canonicalizes its root, checks observed path components for symlinks, bounds object and listing sizes, and uses create-new temporary files for writes. Its standard-library implementation assumes the storage tree cannot be concurrently rewritten by an untrusted local actor; isolate the storage root with appropriate OS permissions.

## Outbound HTTP and notifications

The `client` feature exposes URL/request/response contracts and the bounded `TcpHttpClient`.

- URL parsing recognizes `http` and `https`, rejects userinfo/fragments and invalid authority/path control bytes.
- The built-in TCP transport intentionally sends plaintext `http` only; `https` requires a TLS-capable adapter.
- Request/response header and body sizes plus connect/read/write timeouts are bounded by `TcpClientConfig`.
- Framing ambiguity such as conflicting `Content-Length` or `Transfer-Encoding` plus `Content-Length` is rejected.
- Outbound header values use the same HTAB/printable-ASCII boundary as framework HTTP headers.

The low-level client does not decide which destinations an application trusts. If a URL is influenced by untrusted input, the application must enforce its own scheme, host, port, and network-range policy to prevent SSRF.

Webhook notifications build on these client contracts and expose optional idempotency keys; applications remain responsible for destination trust and side-effect idempotency.

## Optional features

The main `berserk` facade can expose the following optional subsystems: database, Claw ORM, PostgreSQL, MySQL, SQLite, auth, OpenAPI, cache, storage, events, jobs, outbound client, notifications, and CLI tooling. Optional components are feature-gated rather than enabled implicitly.

## Stability and compatibility

- Candidate package version: `0.1.0`.
- Minimum supported Rust version: 1.88.
- No crates.io publication has occurred yet.
- The `0.1.x` API is pre-release and can still change before a stable compatibility commitment.

`README.md`, crate-level Rustdoc, tests, and this contract should agree on public behavior. When implementation and this document diverge, that drift is a release-review finding and must be corrected before publication.
