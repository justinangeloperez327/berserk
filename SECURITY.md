# Security policy

Berserk v1.0.0 is the first stable framework release. The repository has completed a dedicated internal release security/API review pass, but it has not yet completed the independent external security/API review required by the release checklist, nor has it received a third-party penetration test or security certification. Do not describe it as production-certified.

## Security maintainer

The repository owner, `@justinangeloperez327`, is the security maintainer and release-security owner until additional maintainers are explicitly added to `.github/CODEOWNERS` and `docs/vulnerability-response.md`.

## Reporting a vulnerability

Do not disclose exploit details, credentials, private data, or proof-of-concept material in a public issue, discussion, or pull request.

The intended primary private channel is **GitHub Private Vulnerability Reporting** for this repository. Use **Security and quality -> Report a vulnerability** to submit the report privately to the maintainer. GitHub Private Vulnerability Reporting was confirmed enabled for the repository on 2026-09-17.

If the private form is temporarily unavailable, a reporter may open a public issue only to request a private contact method; the issue must contain no vulnerability details.

A complete report should include the affected component/version or commit, impact, attacker prerequisites, reproduction steps or proof of concept, affected configuration, and suggested mitigation when known.

The maintainer targets acknowledgement within 3 business days and initial triage within 7 business days when sufficient reproduction information is available. These are project targets, not contractual service-level agreements. See `docs/vulnerability-response.md` for the full triage, severity, remediation, and disclosure process.

## Supported versions

Berserk v1.0.0 is the current stable baseline. Public API compatibility follows semantic versioning; breaking public API changes require a new major release. Security support and remediation follow the current repository support policy and release documentation.

## Security boundaries

- External input is untrusted and must remain bounded by HTTP, JSON, multipart, database, storage, cache, client, queue, and CLI limits.
- Secret material must not be emitted through default `Debug`, public HTTP errors, request logs, traces, or generated files.
- SQL values must use bindings. Raw SQL is an explicit escape hatch and must never interpolate untrusted strings.
- The built-in server does not terminate TLS. Internet-facing deployments require HTTPS termination through a trusted reverse proxy or a vetted TLS adapter.
- The built-in outbound TCP client intentionally does not implement an SSRF allowlist. Applications that accept user-controlled URLs must enforce allowed schemes, hosts, ports, and network ranges before sending requests.
- Berserk auth currently provides bearer-style session primitives. Applications that place credentials in cookies must define Secure, HttpOnly, SameSite, rotation, revocation, and CSRF policy at the application or adapter layer.
- `MemorySessionStore` is process-local and capacity-bounded (10,000 records by default, configurable with `MemorySessionStore::new`). It is still development-oriented and non-persistent; production deployments should use a persistent store with deployment-appropriate operational limits.
- `LocalStorage` validates normalized paths and rejects observed symlinks, but its standard-library implementation assumes the storage root is not concurrently mutated by an untrusted local actor. Use an isolated root with appropriate OS permissions.
- Synchronous application handlers must return. Network deadlines bound I/O, but the framework cannot safely force-stop arbitrary user code that blocks forever.
- Retries, jobs, events, notifications, and webhooks can duplicate side effects; applications must define idempotency where needed.
- Optional adapters carry their own upstream security and maintenance obligations.

See `docs/security-review.md` for the current review record, findings, evidence, and residual risks.
