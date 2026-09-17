# Security and public API review record

Date: 2026-09-17

Status: internal release review pass completed for the `0.1.0` candidate surface. This is not an independent third-party security audit, penetration test, external certification, or guarantee of vulnerability absence.

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
| SR-08 | Operational boundary | `MemorySessionStore` is process-local and unbounded; arbitrary synchronous handler code can also block indefinitely. | Documented: production systems should use bounded/persistent session storage and must not run unbounded blocking handler work on request paths. |
| SR-09 | Release process | A private vulnerability-reporting channel and named maintainer response ownership are not yet configured. | Remains a release checklist item; this cannot be satisfied by source changes alone. |
| SR-10 | Medium | Framework `Request` `Debug` output included the complete request path, which could expose route-parameter values in logs despite the intended redaction boundary. | Fixed in this review: request diagnostics now report only target byte length plus metadata, with regression coverage that rejects path/query value disclosure. |
| SR-11 | Release process | The repository checklist requires an independent security/API review, but this pass was performed internally while developing the framework. | Kept open: an independent external reviewer must complete that gate before release readiness is claimed. |

## Public API review

The primary API remains coherent around instance registration and explicit execution:

- `App` owns configuration, routes, optional state/database registration, binding and listening.
- `app.route()` is the route registrar; verbs, prefixes, groups, middleware, names, resources and fallback behavior compose without global macros.
- Handler signatures opt into only the inputs they need: typed route parameters, `Request`, `Validated<T>`, and feature-gated Claw route models.
- Input validation order is explicit: decode -> sanitize -> validate.
- Claw query chains build operations while terminal methods perform database I/O.
- Optional subsystems remain feature-gated and the crate prelude exposes common application types without hiding ownership or error boundaries.

The internal review found no additional release-blocking API inconsistency after correcting the stale contract documentation. That conclusion is limited to this internal review and does not replace the independent review gate.

## Residual release gates

This internal review closes the repository's internal hardening pass, but it does not close the independent security/API review requirement. The release checklist also still requires package-name availability, a private vulnerability-reporting channel, supported OS/database-version policy, clean-machine documentation/example review, final changelog/support review, provenance/checksum policy, rollback/yank planning and explicit owner publication approval.

## Conclusion

The reviewed `0.1.0` candidate has a materially stronger and more accurately documented security baseline. Two concrete diagnostic/protocol-boundary issues found by the internal pass were fixed and regression-tested. Remaining findings are explicit deployment or release-process boundaries rather than hidden framework guarantees. An independent external security/API review is still required before that release gate can be marked complete.
