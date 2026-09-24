# Observability core

Berserk's built-in observability contract is vendor-neutral and intentionally low-cardinality.

Request logs use the matched route template rather than the concrete request target. Query strings, route parameter values, headers, bodies, cookies, and credentials are not included by the built-in request logger. Events carry method, status, duration, and available request/trace correlation identifiers.

Trace propagation follows W3C `traceparent`. A valid incoming context retains its trace identifier while Berserk creates a child parent/span identifier. Invalid, duplicate, or absent context produces a new trace. The response carries the resulting `traceparent`.

HTTP metrics use fixed names without user-controlled labels. Requests, active work, total duration, rate limiting, client errors, and server errors can therefore be exported without route/cardinality explosions. The historical `failures` counter remains the 5xx failure count for v1 compatibility; `server_errors` makes that category explicit while `client_errors` records 4xx outcomes.

Active-request accounting must return to zero on success and error paths.

Liveness reports process availability and is independent of external dependencies. Readiness evaluates registered checks and returns 503 when any dependency is unhealthy. Readiness output exposes check names and states, not internal diagnostics.

Database query counts and timings should be measured at the database/Claw boundary rather than inferred from HTTP logs. Slow-query observability must never emit SQL bindings or other secrets. Those hooks can be added without changing the HTTP metrics contract.
