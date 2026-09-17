# Independent security and public API review package

This document is the handoff package for the independent review required before Berserk `0.1.0` can be considered release-ready.

The reviewer must be independent of the implementation work being reviewed. A reviewer may read the existing internal review and test evidence for context, but must not treat its conclusions as proof. The reviewer is expected to inspect the source and reach their own conclusions.

## Review target

Before the review starts, the release owner must record the exact candidate commit SHA below and avoid changing application/runtime source during the review without notifying the reviewer.

- Repository: `justinangeloperez327/berserk`
- Candidate version: `0.1.0`
- Candidate commit: `<record exact reviewed commit>`
- Minimum supported Rust version: `1.88`
- Primary Tier-1 host: Ubuntu 24.04

If source changes are required to resolve a finding, the reviewer must identify whether the changed area needs re-review. The final sign-off must name the final reviewed commit.

## Independence requirement

The release gate is independent only when the reviewer did not author the implementation under review and is not simply repeating the repository's internal review conclusions.

The reviewer may be an individual or organization. The review record must identify the reviewer or reviewing organization, the reviewed commit, review dates, and any material scope limitations.

## Required reading

Start with these files:

1. `README.md` — intended developer experience and examples.
2. `docs/public-api.md` — current public API contract.
3. `SECURITY.md` — supported security boundaries and deployment assumptions.
4. `docs/known-limitations.md` — declared limitations.
5. `docs/security-review.md` — prior internal findings; use as context only.
6. `docs/vulnerability-response.md` — private reporting and remediation process.
7. `docs/release-recovery.md` — release/yank/security incident recovery.
8. `docs/support-policy.md` and `docs/compatibility.md` — supported environments and compatibility policy.

The reviewer should also inspect the relevant tests and workflows rather than relying only on prose documentation.

## Security model

Berserk is an application framework. It accepts untrusted network input and can interact with databases, local storage, outbound network destinations, authentication/session material, application state, jobs, events, caches, notifications, and generated API descriptions.

The review should assume:

- HTTP request methods, targets, headers, query strings, JSON bodies and multipart bodies may be attacker-controlled.
- Route parameters and model identifiers may be attacker-controlled.
- Database values derived from requests may be attacker-controlled.
- Storage object paths may be attacker-controlled unless an application constrains them.
- Outbound URLs may be attacker-influenced unless an application constrains them.
- Authentication tokens, session identifiers and password inputs are sensitive.
- Log and diagnostic output may be observable by operators or external log systems and must not expose secrets by default.
- Framework users may make configuration mistakes; APIs should reject invalid states where practical and document deployment responsibilities where they cannot.

The following are explicit deployment boundaries rather than hidden framework guarantees:

- the built-in server does not terminate TLS;
- the built-in outbound client does not implement an application SSRF allowlist;
- cookie attributes and CSRF policy are application responsibilities;
- `MemorySessionStore` is process-local and unbounded;
- `LocalStorage` assumes an untrusted local actor cannot concurrently rewrite its storage tree;
- arbitrary synchronous application code cannot be force-stopped safely by the framework.

A finding should be raised if implementation behavior is weaker than these documented boundaries, if the documentation materially understates a risk, or if a safer public API boundary is reasonably achievable.

## Mandatory security review areas

### 1. HTTP parsing, routing and request extraction

Review the framework/core HTTP path from socket input to handler invocation.

Verify at minimum:

- request-line, target, header and body limits cannot be bypassed through alternate framing or malformed input;
- invalid control bytes and framing ambiguity are rejected safely;
- route matching cannot confuse static and parameter routes in a way that bypasses authorization assumptions;
- method handling, `HEAD`, fallback, `404`, `405` and `Allow` behavior are coherent;
- route parameter parsing fails closed;
- query, JSON and multipart parsing are bounded and return controlled errors rather than panics or unintended allocations;
- multipart filenames or metadata are not automatically trusted as filesystem paths;
- handler extraction cannot accidentally deliver unvalidated input where `Validated<T>` is promised.

Priority areas: `crates/core`, `crates/framework`, their tests, and HTTP/routing fuzz targets.

### 2. Sanitization and validation order

Confirm the documented order is actually enforced for validated handler input:

`decode -> sanitize -> validate -> handler`

Check that malformed decoding, sanitizer failures if applicable, and validation failures cannot fall through into controller execution. Verify error responses do not expose sensitive internals.

Priority areas: `crates/validation`, framework handler/extractor code and controller tests.

### 3. Database and Claw ORM

Review normal query construction, raw SQL escape hatches, request-scoped connections, transactions, model persistence and route-model binding.

Verify at minimum:

- normal query APIs separate values from SQL text and do not interpolate attacker-controlled values;
- identifier handling does not create an injection path where values are expected;
- `where_`, `or_where`, `where_in`, ordering, pagination and terminal operations preserve binding safety;
- update/delete/save operations cannot silently broaden a write filter;
- primary-key handling cannot accidentally overwrite or persist forbidden fields;
- request-scoped transaction commit/rollback semantics fail safely;
- nested transaction behavior matches documentation;
- route-model and scoped nested binding cannot return an object outside the intended parent scope;
- database errors and `Debug` output do not leak SQL/bind values by default.

Priority areas: `crates/database`, `crates/claw`, database contract tests and live PostgreSQL/MySQL/SQLite workflow coverage.

### 4. Authentication, sessions and authorization

Review password handling, secret wrappers, session generation/storage, principal extraction, guards and authorization contracts.

Verify at minimum:

- password hashing uses a suitable password-hashing API and never logs plaintext password material;
- session tokens use cryptographically secure randomness;
- provided stores do not retain plaintext bearer tokens where the documented contract says digests are stored;
- expiration, revocation and pruning behave correctly;
- equality/comparison behavior does not introduce obvious secret disclosure;
- `Debug`/error output redacts sensitive values;
- missing/invalid authentication fails closed;
- authorization helpers do not accidentally default to allow.

Priority areas: `crates/auth`, framework auth extraction/middleware and their tests.

### 5. Storage

Review `StoragePath`, in-memory storage and local filesystem storage.

Verify at minimum:

- absolute paths, traversal, ambiguous separators, NUL/control bytes and empty components are rejected;
- encoded or normalized forms cannot bypass path rules;
- symlink handling matches the documented threat model;
- temporary-write/replace behavior cannot corrupt an existing object after a failed oversized write;
- object size and listing limits are effective;
- error messages and diagnostics do not expose more local filesystem detail than necessary.

Priority areas: `crates/storage` and storage contract tests.

### 6. Outbound HTTP, notifications and SSRF boundary

Review URL parsing, headers, framing, redirects if present, timeouts, body limits and notification/webhook behavior.

Verify at minimum:

- invalid authorities, userinfo/fragments where prohibited, control bytes and malformed headers are rejected;
- conflicting framing metadata cannot create request smuggling/desynchronization behavior;
- configured connect/read/write/body limits are effective;
- plaintext `http` versus TLS-capable behavior is not misleading to callers;
- user-controlled destinations are clearly an application trust decision and no helper accidentally bypasses that boundary;
- retry/notification behavior does not silently promise exactly-once delivery.

Priority areas: `crates/client`, `crates/notifications` and their tests.

### 7. Server concurrency, overload and shutdown

Review queue/worker limits, timeout handling, graceful shutdown and error paths.

Verify at minimum:

- overload is bounded and rejected deliberately;
- accepted requests are not silently dropped during graceful shutdown beyond documented behavior;
- network deadlines do not leave obvious unbounded resource retention;
- panic/error paths do not poison shared framework state unsafely;
- synchronous handler limitations are documented accurately.

Priority areas: framework server code, concurrent-load tests, overload tests and prolonged-soak evidence.

### 8. Cache, jobs, events and side effects

Review boundaries that may duplicate or defer side effects.

Verify at minimum:

- cache keys/values are redacted in default diagnostics where promised;
- job/event retry behavior does not claim exactly-once semantics unless it actually provides them;
- attacker-controlled payload sizes or retry counts are bounded where the framework is responsible;
- idempotency responsibilities are documented for operations that may execute more than once.

Priority areas: `crates/cache`, `crates/jobs`, `crates/events` and integration tests.

### 9. OpenAPI, CLI and generated material

Review generated files and descriptions for injection, unsafe paths and accidental secret capture.

Verify at minimum:

- generated files cannot escape the intended destination through user-controlled names;
- generated source/configuration does not include credentials or local machine secrets;
- OpenAPI output does not expose private runtime data;
- CLI input validation fails clearly on unsupported names/paths.

Priority areas: `crates/openapi`, `crates/cli`, package-content checks and generator tests.

### 10. Diagnostics and secret redaction

Actively attempt to make sensitive material appear in `Debug`, errors, logs, traces or generated artifacts.

Include request targets/query values, authorization headers, cookies, passwords, bearer/session tokens, SQL/bindings, cache keys/values, multipart bodies, local paths and outbound credentials where applicable.

A default diagnostic path that leaks a secret or high-value attacker-controlled value should be treated as a security finding even if applications could avoid logging it manually.

### 11. Dependency and unsafe-code boundary

Verify:

- library crates preserve the intended `unsafe_code = "forbid"` policy where documented;
- optional features do not unexpectedly weaken a security boundary;
- dependency advisories/source/license checks cover the actual release dependency graph;
- feature combinations do not expose an unreviewed alternate code path.

Review `Cargo.toml`, `Cargo.lock`, workspace features and `.github/workflows/security.yml` / `ci.yml`.

## Mandatory public API review areas

The API review is not a style preference exercise. Focus on whether the public surface creates ambiguity, accidental misuse, hidden runtime behavior or commitments the implementation does not satisfy.

Review at minimum:

- `App` assembly and server lifecycle;
- route registration, grouping, naming, resources and reverse routing;
- handler/controller signatures and extraction rules;
- `Request`, `Response`, JSON/query/multipart and `Validated<T>` behavior;
- middleware ordering and scope;
- database/Claw query and persistence ergonomics;
- authentication/session/authorization contracts;
- storage/client/notification contracts;
- feature gating and prelude exports;
- error types and whether recoverable application mistakes require unnecessary internal knowledge;
- README examples versus real compilable API;
- documented MSRV and platform/database support versus public API requirements.

For every API issue, state whether it is:

- a release-blocking correctness/safety problem;
- a compatibility concern that should be fixed before `0.1.0`;
- a documentation mismatch;
- or a non-blocking ergonomic suggestion.

## Adversarial test expectations

The reviewer is encouraged to add temporary or permanent tests. High-value cases include malformed request framing, control-byte headers, oversized bodies, route collisions, malformed percent encodings, multipart boundary abuse, SQL metacharacters in bound values, transaction failure paths, invalid session tokens, storage traversal/symlink cases and outbound framing ambiguity.

Any newly discovered vulnerability or correctness defect should receive a permanent regression test when practical.

## Existing evidence index

The repository already contains evidence that the reviewer may use to prioritize work, but not as a substitute for source inspection:

- `.github/workflows/ci.yml` — formatting, compile, Clippy, tests, docs, feature combinations, MSRV and Windows/macOS coverage;
- `.github/workflows/databases.yml` — live PostgreSQL, MySQL and SQLite coverage;
- `.github/workflows/security.yml` — advisory/license/source policy checks;
- `.github/workflows/fuzz.yml` — JSON, HTTP values, route registration and multipart fuzzing;
- `.github/workflows/load.yml` — overload, shutdown-under-load and soak coverage;
- `.github/workflows/benchmarks.yml` — reproducible performance evidence;
- `.github/workflows/package.yml` — package metadata/content and clean external-consumer checks;
- `.github/workflows/release-artifacts.yml` — release evidence/checksum/provenance construction;
- `docs/security-review.md` — prior internal review findings and residual boundaries;
- `docs/known-limitations.md` — declared limitations that should be challenged for accuracy.

## Finding severity

Use these severities unless the reviewer has a documented equivalent methodology:

- **Critical** — practical compromise with severe confidentiality, integrity or availability impact, such as remote code execution or broad authentication bypass.
- **High** — serious compromise with limited prerequisites, including significant authorization bypass, exploitable injection or strong remotely triggerable denial of service.
- **Medium** — meaningful impact with material prerequisites, reduced scope or substantial attacker constraints.
- **Low** — limited-impact weakness or defense-in-depth gap with low practical exploitability.
- **Documentation/API** — no direct vulnerability demonstrated, but the contract, examples or API shape are materially misleading or unsafe to rely upon.
- **Informational** — useful hardening or ergonomics suggestion that does not block release.

The reviewer should include exploitability, affected configuration, affected crates and affected versions/commit in each finding.

## Required deliverables

The independent review is complete only when the reviewer provides:

1. reviewer identity or organization;
2. review start/end dates;
3. exact final reviewed commit SHA;
4. scope reviewed and any exclusions;
5. methodology and tests/tools used;
6. findings using `docs/external-review-findings-template.md` or equivalent detail;
7. an explicit statement for each mandatory review area indicating either findings or no material finding observed;
8. re-review notes for any fixes made during the review;
9. final residual risks/limitations the reviewer believes should remain documented; and
10. reviewer sign-off using `docs/external-review-signoff-template.md` or equivalent wording.

## Release-gate closure criteria

Do **not** close the independent-review release gate merely because review work started or because a reviewer reports no severe vulnerabilities informally.

The gate may close only when:

- the final reviewed commit is recorded;
- every Critical/High finding is fixed, or the release owner records a specific residual-risk decision and the reviewer has seen the disposition;
- every Medium finding is fixed or explicitly documented with disposition;
- documentation/API findings that materially affect the published contract are corrected or explicitly accepted;
- regression tests are added for corrected defects when practical;
- all source changes made for findings have passed the relevant CI/security/database/fuzz/load/package checks;
- the reviewer provides a final sign-off and scope limitations; and
- `docs/external-review-record.md` is created from the sign-off/findings material and linked from `docs/release-checklist.md`.

An external review reduces risk; it is not a certification that Berserk is vulnerability-free.
