# Security and public API review record

Date: 2026-09-17

Status: this file contains the historical internal review from 2026-09-17 plus the V1 internal pre-review refresh from 2026-09-22. It is development evidence only. The owner elected to publish v1.0.0 without a pre-release independent API/security review. This record is not a penetration test, security certification, or guarantee of vulnerability absence.

## Scope

The review covered the security-sensitive and developer-facing boundaries of the current workspace:

- application assembly, routing, controller extraction, middleware, HTTP request/response handling, JSON, query decoding, multipart parsing, server limits, overload and shutdown behavior;
- authentication primitives, password hashing, session/token handling and authorization contracts;
- database/Claw query boundaries, bound values, raw SQL escape hatches, request-scoped connections and transactions;
- storage path handling and local storage behavior;
- outbound HTTP URL/header validation and transport limits;
- cache, jobs, events, notifications, CLI generation and diagnostic redaction boundaries;
- public exports, prelude, README examples, release metadata and security documentation;
- automated evidence already retained by CI, dependency policy, fuzzing, live-database, package, performance, concurrent-load and prolonged-soak workflows.

## Verified controls

- Library crate roots forbid unsafe code.
- HTTP metadata and bodies are bounded; server queueing, timeouts and overload behavior are explicit.
- Framework request/response header values use a strict text subset and reject prohibited control bytes.
- JSON and multipart parsing are bounded, and multipart parsing does not trust filenames or write to the filesystem.
- Typed request input is decoded, sanitized, then validated before controller delivery through `Validated<T>`.
- Route model binding returns controlled 400/404 responses and scoped nested binding constrains child lookup through the parent model.
- Database values remain separate from SQL text in normal query APIs; raw SQL remains an explicit escape hatch.
- Request-scoped transactions have explicit commit/rollback behavior and reject unsupported nested request transactions.
- Session tokens use OS randomness, are stored by digest, redact `Debug`, expire, and support revocation.
- Password operations use the RustCrypto Argon2 password-hashing API and keep password material behind a redacting secret wrapper.
- Storage paths reject absolute paths, traversal, empty components, backslashes and control bytes; local storage rejects observed symlinks.
- Default diagnostics redact request targets/bodies, header values, SQL/bindings, cache keys/values, multipart bodies and authentication secrets.
- Dependency advisory/license/source policy, fuzzing, real PostgreSQL/MySQL/SQLite tests, sequential performance evidence, concurrent overload/shutdown tests, and a 900-second main-branch soak have completed successfully.

## Findings

| ID | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| SR-01 | Medium | Outbound client header values rejected CR/LF/NUL but still accepted other ASCII control bytes and non-ASCII bytes, making the client boundary weaker than the framework HTTP boundary. | Fixed in this review: outbound `Header` now permits only HTAB and printable ASCII, with regression tests for C0 controls, DEL, non-ASCII text and allowed HTAB. |
| SR-02 | Documentation | `SECURITY.md` still described the workspace as `0.0.0` with every crate non-publishable. | Fixed: policy now reflects the planned `0.1.0` publishable package surface. |
| SR-03 | Documentation/API | `docs/public-api.md` still described implemented routing groups, middleware, validation, database, authentication and CLI capabilities as deferred. | Fixed: the contract is rewritten against the current `0.1.0` pre-release API. |
| SR-04 | Deployment boundary | The built-in server does not terminate TLS. | Documented: production/internet-facing deployments require a trusted HTTPS reverse proxy or vetted TLS adapter. |
| SR-05 | Deployment boundary | The outbound client does not decide which destinations are trusted, so user-controlled URLs can create SSRF exposure in an application. | Documented: applications must enforce scheme/host/port/network allowlists before sending untrusted URLs. |
| SR-06 | Deployment boundary | `LocalStorage` performs normalized-path and symlink checks using standard filesystem operations, which cannot eliminate every local TOCTOU race if an untrusted actor can mutate the storage tree concurrently. | Documented: the storage root must be isolated and writable only by the application/trusted operators. |
| SR-07 | Deployment boundary | Bearer session primitives do not automatically define cookie attributes or CSRF policy. | Documented: cookie-based applications must define Secure/HttpOnly/SameSite, rotation/revocation and CSRF behavior. |
| SR-08 | Operational boundary | Historical finding: `MemorySessionStore` was described as process-local and unbounded; arbitrary synchronous handler code can also block indefinitely. | **Superseded in part by SR-14.** The current store is capacity-bounded (10,000 records by default) but remains process-local and non-persistent. Arbitrary blocking handler work remains an application/runtime boundary. |
| SR-09 | Release process | Historical finding: a private vulnerability-reporting channel and named maintainer response ownership were not yet configured. | **Resolved later.** GitHub Private Vulnerability Reporting and maintainer ownership are now documented in `SECURITY.md`, `.github/CODEOWNERS`, and `docs/vulnerability-response.md`. |
| SR-10 | Medium | Framework `Request` `Debug` output included the complete request path, which could expose route-parameter values in logs despite the intended redaction boundary. | Fixed in this review: request diagnostics now report only target byte length plus metadata, with regression coverage that rejects path/query value disclosure. |
| SR-11 | Release process | The repository originally required an independent API/security review, but this pass was performed internally while developing the framework. | Deferred for v1.0.0 by explicit owner decision. This is a risk acceptance, not an independent review. |

## V1 pre-review refresh — 2026-09-22

This refresh was performed after the Claw eager-loading, Axe production-view,
and foundation reference-application maturity work was merged. It is an
**internal remediation pass only**. It does not satisfy the independent review
gate.

| ID | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| SR-12 | Medium | `RequestLogger` recorded the concrete request path. Although query strings were excluded, route-parameter values such as account identifiers, opaque IDs, or secrets embedded in path segments could enter logs. | Fixed on `api-security-review-prep`: routing records the matched route template in request-local shared state; `RequestLogger` emits that template (for example `/users/{id}`) or a fixed unmatched/fallback marker. Regression tests assert that route-parameter and query values are absent. |
| SR-13 | Low / availability | `LocalStorage` temporary-name generation used `OsRng.fill_bytes`, whose infallible wrapper can panic if operating-system entropy is unavailable. Storage operations should fail through the storage error boundary instead of terminating request work. | Fixed on `api-security-review-prep`: temporary-name generation uses `try_fill_bytes` and returns a controlled `StorageError`. |
| SR-14 | Documentation | Security and limitation docs still called `MemorySessionStore` unbounded even though the current implementation has a configurable positive capacity and a 10,000-record default. | Fixed: documentation now describes the actual bounded, process-local, non-persistent behavior. |
| SR-15 | Release process | No independent API/security review was completed before v1.0.0. Internal review, CI, fuzzing, and this remediation pass do not substitute for one. | Accepted for v1.0.0 by owner decision and disclosed publicly; a post-release independent review remains recommended. |

The earlier SR-08 statement that the memory session store was unbounded is
historical. The current implementation is capacity-bounded; its remaining
production limitation is process-local, non-persistent storage. SR-09 is also
historical: GitHub Private Vulnerability Reporting and maintainer ownership are
now documented in `SECURITY.md` and `docs/vulnerability-response.md`.

## Public API review

The primary API remains coherent around instance registration and explicit execution:

- `App` owns configuration, routes, optional state/database registration, binding and listening.
- `app.route()` is the route registrar; verbs, prefixes, groups, middleware, names, resources and fallback behavior compose without global macros.
- Handler signatures opt into only the inputs they need: typed route parameters, `Request`, `Validated<T>`, and feature-gated Claw route models.
- Input validation order is explicit: decode -> sanitize -> validate.
- Claw query chains build operations while terminal methods perform database I/O.
- Optional subsystems remain feature-gated and the crate prelude exposes common application types without hiding ownership or error boundaries.

The internal review found no additional release-blocking API inconsistency after correcting the stale contract documentation. That conclusion is limited to internal review and must not be described as an independent audit.

## Residual release gates

Internal review and remediation do not constitute an independent API/security review. For v1.0.0, the owner explicitly deferred that review and accepted the residual risk. The remaining release gates are exact-commit validation, publication-graph verification, provenance, final documentation review, and owner approval.

## Conclusion

The historical 2026-09-17 review and the 2026-09-22 V1 pre-review refresh have
produced concrete hardening changes and corrected documentation drift. The
current internal pass found and remediated route-parameter disclosure in
built-in request logs and a panic-capable LocalStorage entropy path. These
internal results are preparation evidence only. The independent API/security
review remains open and must verify the exact candidate plus any remediation
before the framework-maturity gate can be marked complete.
