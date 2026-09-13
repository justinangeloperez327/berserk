# Security policy

This repository is a pre-release framework prototype and has not received an independent security audit. Do not treat it as production-ready.

## Reporting a vulnerability

Do not open a public issue containing exploit details, credentials, or private data. Before publishing the project, replace this paragraph with a private security contact or enable private vulnerability reporting on the chosen forge. A complete report should include the affected component, impact, reproduction steps, and suggested mitigation when known.

No response-time commitment exists until a maintainer and private reporting channel are named.

## Supported versions

There are no supported releases yet. The workspace version is `0.0.0`, every package has `publish = false`, and compatibility may change while the design is validated.

## Security boundaries

- External input is untrusted and must remain bounded by HTTP, JSON, multipart, database, storage, cache, and CLI limits.
- Secret material must not be emitted through default `Debug`, public HTTP errors, request logs, traces, or generated files.
- SQL values must use bindings. Raw SQL is an explicit escape hatch and must never interpolate untrusted strings.
- Authentication does not replace transport security. A production deployment requires HTTPS termination and secure cookie/session configuration.
- Retries, jobs, events, notifications, and webhooks can duplicate side effects; applications must define idempotency where needed.
- Optional adapters carry their own upstream security and maintenance obligations.

See `docs/security-review.md` for the current audit record and unresolved risks.
