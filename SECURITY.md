# Security policy

Berserk is a pre-release framework. The repository has completed a dedicated internal release security/API review pass, but it has not yet completed the independent external security/API review required by the release checklist, nor has it received a third-party penetration test or security certification. Do not describe it as production-certified.

## Reporting a vulnerability

Do not open a public issue containing exploit details, credentials, or private data. Before public package release, the maintainer must enable a private vulnerability-reporting channel or publish a private security contact. A complete report should include the affected component, impact, reproduction steps, and suggested mitigation when known.

No response-time commitment exists until a maintainer and private reporting channel are named.

## Supported versions

There are no published supported releases yet. The planned initial package version is `0.1.0`; publishable library and CLI crates are prepared, while examples and benchmarks remain non-publishable. Compatibility may still change before the first stable release.

## Security boundaries

- External input is untrusted and must remain bounded by HTTP, JSON, multipart, database, storage, cache, client, queue, and CLI limits.
- Secret material must not be emitted through default `Debug`, public HTTP errors, request logs, traces, or generated files.
- SQL values must use bindings. Raw SQL is an explicit escape hatch and must never interpolate untrusted strings.
- The built-in server does not terminate TLS. Internet-facing deployments require HTTPS termination through a trusted reverse proxy or a vetted TLS adapter.
- The built-in outbound TCP client intentionally does not implement an SSRF allowlist. Applications that accept user-controlled URLs must enforce allowed schemes, hosts, ports, and network ranges before sending requests.
- Berserk auth currently provides bearer-style session primitives. Applications that place credentials in cookies must define Secure, HttpOnly, SameSite, rotation, revocation, and CSRF policy at the application or adapter layer.
- `MemorySessionStore` is process-local and unbounded; production deployments with untrusted session creation should use a bounded/persistent store with operational limits.
- `LocalStorage` validates normalized paths and rejects observed symlinks, but its standard-library implementation assumes the storage root is not concurrently mutated by an untrusted local actor. Use an isolated root with appropriate OS permissions.
- Synchronous application handlers must return. Network deadlines bound I/O, but the framework cannot safely force-stop arbitrary user code that blocks forever.
- Retries, jobs, events, notifications, and webhooks can duplicate side effects; applications must define idempotency where needed.
- Optional adapters carry their own upstream security and maintenance obligations.

See `docs/security-review.md` for the current review record, findings, evidence, and residual risks.
