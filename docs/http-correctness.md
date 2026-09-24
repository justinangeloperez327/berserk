# HTTP correctness contract

Berserk treats the HTTP path as one ordered lifecycle:

`Request → route selection → middleware → authentication/authorization when configured → typed extraction and route-model binding → input sanitation/validation → controller → Claw/database → Response`.

Route selection is based on the path, not the query string. Static route specificity wins over parameter routes independently of registration order. Method selection occurs after the route pattern is selected. A method mismatch returns 405 with an `Allow` header for that selected pattern; GET implies HEAD unless an explicit HEAD route exists.

Typed route-parameter parse failures are 400. Missing routes and missing bound models are 404. Malformed JSON is 400, unsupported request media type is 415, and semantic validation failures are 422. Successful creation endpoints may return 201 and bodyless deletion endpoints 204.

HEAD uses explicit HEAD when registered, otherwise GET representation metadata. The response body is always suppressed while representation length is retained.

The full-lifecycle tests intentionally cross subsystem boundaries. They complement focused router, middleware, authentication, model-binding, request-transaction, transport, overload, shutdown, header/body-limit, and HTTP security tests already present in the framework.

HTTP correctness changes must preserve the v1 compatibility firewall. New convenience APIs are not a substitute for deterministic lifecycle behavior.
