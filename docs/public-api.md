# Public API contract

Status: proposed implementation target, not working library code.

## Application

- `App::new() -> App`: default settings.
- `App::with_config(ServerConfig) -> Result<App>`: validate custom settings.
- `get`, `post`, `put`, `patch`, `delete`, `head`, `options`: register a path and handler, returning `Result<()>`.
- `route(method, path, handler) -> Result<()>`: explicit method registration.
- `listen(address) -> Result<()>`: consume the app, bind, run synchronously.
- `bind(address) -> Result<Server>`: bind without starting the run loop.

Handlers accept an owned Request and return Response or Result<Response>. Registration conceptually requires `F: Fn(Request) -> R + Send + Sync + 'static` and `R: IntoResponse`. Closures and ordinary functions both work. Different worker threads may execute handlers concurrently.

## Routing

- Parameters use `{name}` and capture exactly one nonempty segment.
- Static segments take precedence over parameter segments, compared from left to right.
- Select the most specific matching path before selecting its method; registration order does not decide precedence.
- `/users` and `/users/` are distinct. Query strings do not participate in matching.
- Equivalent parameter patterns registered for the same method are rejected, even when parameter names differ.
- Missing paths return 404. A matched path with no matching method returns 405 and an Allow header.
- Explicit HEAD takes priority; otherwise fall back to GET. Suppress bodies for both. Allow includes HEAD when GET is available.
- OPTIONS is explicit initially; no automatic CORS behavior.
- Reject malformed patterns, repeated parameter names within a route, and unsupported wildcard syntax during registration.

## Request

| Method | Return | Meaning |
| --- | --- | --- |
| method() | &Method | HTTP method |
| path() | &str | Raw path without query |
| param(name) | Option<&str> | Raw captured segment |
| query_string() | Option<&str> | Raw query string |
| header(name) | Option<&str> | First header value; case-insensitive name |
| headers() | &Headers | All header values, preserving repetitions |
| body() | &[u8] | Buffered body |
| text() | Result<&str> | Validated UTF-8 body |

Fields remain private. URL decoding will be a separate explicit operation. Initial header value representation supports a documented validated text subset; the parser must reject unsupported bytes rather than silently corrupt them. Framing validation examines every header occurrence, not just header(name).

## Response

- `Response::text(value)`: owned UTF-8 body, text/plain; charset=utf-8.
- `Response::bytes(value)`: owned bytes, application/octet-stream.
- `Response::empty()`: no body.
- Default status: 200.
- `.status(code) -> Response`: retain ergonomic builder; validate supported status before encoding.
- `.header(name, value) -> Result<Response>`: validate and replace matching header values.
- `.append_header(name, value) -> Result<Response>`: validate and append.
- Content-Length, Transfer-Encoding, and Connection are encoder-owned; custom attempts are errors.
- Validate all metadata before any response bytes are written. Invalid response configurations become a generic 500 if no bytes have been sent.
- Initially support final responses in 200..=599. Protocol upgrades and informational response sequences are deferred.
- HEAD never sends body bytes; representation length may be advertised. 204 and 304 require status-specific framing; 205 is empty. Encode these using dedicated tests, not one generic empty-body rule.

`Response::text(req.text()?)` must copy the borrowed body into owned response storage so the response outlives the request.

## Error behavior

Setup errors: invalid routes, duplicates, invalid settings. Server errors: bind and listener failures. Client errors: malformed requests and invalid UTF-8 requested as text. Unexpected handler failures: generic 500 without internal details. Expected application outcomes may be explicit responses.

Keep core, routing, HTTP, and server failures distinguishable. Panic containment requires unwinding; panic=abort cannot be caught. Disconnects terminate their connection, not the server. Observability integrations come later; internal errors should remain available to a diagnostic boundary.

## Server and shutdown

Plain TCP, synchronous handlers, one request per connection, Connection: close. Accept origin-form HTTP/1.1 requests initially. Validate required Host and Content-Length; reject unsupported transfer encoding, ambiguous framing, and unsupported expectations. Never trust a size before validating it against limits.

Configuration covers workers, queue capacity, header bytes/count, body bytes, read deadlines, and write deadlines. Use total request deadlines as well as appropriate socket timeouts to prevent indefinitely slow clients. Exact defaults and overload rejection behavior must be chosen and tested before server implementation is declared complete.

`server.shutdown_handle()` returns a cloneable handle. `shutdown.shutdown()` stops new acceptance; `server.run()` drains accepted work and returns when workers finish. Network deadlines bound I/O waits. Rust cannot safely force-stop arbitrary synchronous user code: a handler that never returns can prevent draining from completing. Standard-library-only mode does not promise portable OS-signal integration.

## Deferred

Middleware chains, route groups, typed extraction, JSON, validation, database, authentication, TLS, async handlers, CLI generation. Core design must allow these without requiring application folder conventions.
