# Phase 4 checkpoint

## Implementation

Added routing/error.rs, route.rs and mod.rs. App now owns a private Router and exposes get/post/put/patch/delete/head/options, route(Method, path, handler), and handle(Request). Registration validates before mutation. Handler storage uses boxed Send + Sync closures returning Result<Response>; no async runtime or third-party dependency.

Patterns split at slash and preserve empty static segments. Parameters require ASCII identifier names and capture exactly one nonempty raw segment. Static route grammar is validated through the existing Request target rules. Query, fragment, wildcard and malformed brace patterns are rejected. Equivalent routes ignore parameter names only for conflict detection; dispatch captures using the selected method's own names.

Matching selects the lexicographically most specific static/parameter sequence before method lookup. Equivalent method variants form one path family. Consequently GET /users/new returns 405 if only POST /users/new exists, even when GET /users/{id} exists. Allow is alphabetically ordered and adds HEAD for GET. OPTIONS is explicit. Lowercase extension method tokens remain distinct.

HEAD keeps the incoming method as HEAD even during GET fallback, validates the handler response, clears body bytes and retains representation_length. Router-generated 404/405 responses are also suppressed. Invalid handler responses and errors propagate before suppression. Wire error rendering and bodyless-status framing are deferred.

## Review

Retained foundation and HTTP tests. Added seven routing tests covering handler forms, failures, precedence/order, registration atomicity, trailing slashes, Allow, OPTIONS, HEAD, encoded parameters, invalid responses and concurrent sharing. These are test sources, not executed evidence. Manifests and archive checked; compiler and runtime checks remain pending because Rust tooling is unavailable.

The baseline matcher scans registered routes and allocates temporary captures during selection. This favors readable correctness initially; no performance claims. Measure before optimizing in the performance phase.

## Remaining scope

Phase 5 adds parsing/encoding. Server execution and panic containment follow. Route groups and middleware remain Phase 9. The main application can already be organized in any files; no folder discovery is used.
