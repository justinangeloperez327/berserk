# Implementation acceptance checks

These checks define the release target. The workflow results for the exact release-candidate commit are the source of truth; results from an earlier commit do not satisfy a later candidate. Independent review and owner release authorization remain explicit non-automated gates.

## Public API

- A separate consumer compiles the documented example.
- Named functions and closures accept Request and return Response or Result<Response>.
- Non-thread-safe captures fail at compile time.
- Response bodies own their bytes after the Request is dropped.
- No folder discovery, base classes, or macros are required.

## Routing

- Static/parameter precedence, method selection, distinct trailing slashes.
- Invalid patterns and equivalent duplicate routes fail deterministically.
- 404 versus 405 and accurate Allow values.
- Explicit HEAD and GET fallback suppress wire bodies with correct headers.
- Raw encoded segments and query strings do not alter matching unexpectedly.

## Parsing and encoding

- Partial reads, truncated bodies, premature EOF, malformed request lines and headers.
- Missing/duplicate/invalid Host; repeated/conflicting Content-Length; transfer encoding combinations.
- Header and body limits are enforced before unbounded allocation.
- CR/LF header injection is rejected. Encoder-owned headers cannot be overridden.
- Correct byte lengths for Unicode and binary bodies; dedicated HEAD, 204, 205, 304 tests.
- Unsupported framing and expectations fail promptly; one response per connection.

## Reliability

- Slow clients cannot occupy workers indefinitely through byte-by-byte progress.
- Queue saturation does not create extra unbounded workers or block the accept loop on lengthy rejection writes.
- Disconnects and unwinding handler panics do not stop the listener.
- Shutdown stops accepting and drains bounded I/O work; document nonreturning handler limitation.
- Failed startup releases resources and returns useful errors.

## Database contracts

- SQL text and bound values remain separate at every execution boundary.
- Rows preserve column order and reject empty or duplicate column names.
- Connection and transaction implementations can be substituted behind shared contracts.
- Commit and rollback consume a transaction handle so it cannot be reused.
- Unsupported backend capabilities return a structured database error.
- Default builds contain no database driver; each backend feature is independently selectable.
- PostgreSQL, MySQL, and SQLite drivers must later pass the same contract suite plus backend-specific tests.

## PostgreSQL driver

- The feature-disabled workspace does not compile or link the PostgreSQL dependency.
- SQL bindings use the extended query API and are never interpolated into SQL text.
- Supported scalar values round-trip without silent precision loss.
- Unsupported and untyped values fail explicitly before or during decoding.
- SQLSTATE codes survive error mapping and major error classes remain distinguishable.
- Commit and rollback operate on the same borrowed connection and consume the transaction.
- TLS is never silently disabled; the no-TLS constructor is explicit.
- Opt-in live tests pass against a dedicated supported PostgreSQL instance.

## MySQL and SQLite drivers

- Each driver is absent unless its independent feature is enabled.
- MySQL uses prepared execution for bound values and secure transport support is compiled in.
- MySQL preserves native unsigned values and last-insert IDs without applying those semantics to other databases.
- SQLite file and memory connections pass the same execution and transaction contracts.
- SQLite rejects integers outside its signed 64-bit storage range.
- Text and binary results remain distinguishable; unsupported precision-sensitive values fail explicitly.
- Transactions commit or roll back explicitly, and dropping an unfinished transaction rolls it back through the driver.
- MySQL live tests pass against a dedicated database; SQLite in-memory tests run without external services.

## Fluent query builder

- Identical query chains preserve binding order across all supported dialects.
- PostgreSQL, MySQL, and SQLite emit their correct identifier quoting and placeholders.
- `where_`, OR, IN, NULL, joins, ordering, limit, and offset compose deterministically.
- Select, insert, update, delete, and raw bound execution use the proper connection terminal method.
- Invalid identifiers/operators and ambiguous NULL comparisons fail before execution.
- Unfiltered update/delete requires the visible `allow_all()` opt-in.
- Generated SQL tests and SQLite execution tests cover the portable behavior.

## Axe views

- Named view compilation walks `.html` files deterministically and rejects invalid view names.
- Build-time validation uses the same compiler and dependency rules as runtime rendering.
- Template syntax failures identify the originating view with one-based line and column information.
- Root-relative `@include("...")` dependencies render through the compiled view tree.
- Missing include targets, cycles, root traversal, and excessive include depth fail before rendering.
- Release rendering caches one compiled view set for the selected root instead of reparsing templates per request.
- Debug rendering continues to observe template edits without requiring a process restart.
- Symlinked view entries are not followed outside the configured root.

## Models and relationships

- Model queries decode every returned row through an explicit `Model::from_row` implementation.
- Missing columns, NULL mismatches, incompatible types, and lossy integer casts return decode errors.
- Named and inline scopes compose query builders without executing database I/O.
- `all`, `find`, `get`, and `first` are visible model-query execution boundaries.
- `has_many`, `has_one`, and `belongs_to` eager loading batches nonempty relationship loads into bounded key queries rather than one query per parent.
- Eager key batches never exceed the Claw internal safety threshold; many-to-many loading applies the same bound independently to pivot and related-model lookups.
- Empty parent collections do not execute a query; duplicate and NULL relationship keys are handled deterministically.
- Eager-loaded records are grouped by linking key without mutating models or requiring model cloning.
- `has_one` reports cardinality violations rather than silently discarding rows.
- No relationship performs implicit lazy loading.

## Database tooling

- Migration names are nonempty and unique before any migration executes.
- Applied migrations are skipped and successful new migrations share a new tracked batch.
- Migration tracking is written only after all `up` statements for that migration succeed.
- The newest migration batch rolls back in reverse recorded order.
- Missing applied migration definitions fail clearly instead of silently changing history.
- Cross-driver DDL transaction guarantees are not claimed; partial DDL failure is documented.
- Seeders run in caller-supplied order and stop at the first error.
- Factory generation is deterministic from its index and performs no hidden persistence.
- Pagination validates bounds and detects offset overflow.
- Pagination runs one filtered count query and one bounded data query.
- Page metadata handles empty results and final pages consistently.

## Authentication and authorization

- Migration-independent identity providers return principals and password hashes through explicit contracts.
- Argon2 hashes use fresh operating-system-generated salts and PHC encoding.
- Unknown, disabled, and incorrect-password accounts return one generic credential failure.
- Unknown identifiers still perform password verification against a dummy hash.
- Passwords and raw tokens redact debug output and zeroize their owned memory on drop.
- Session tokens contain 256 random bits and only token digests reach session stores.
- Expiration, revocation, pruning, malformed tokens, and store failures behave explicitly.
- Bearer middleware challenges missing or rejected credentials and attaches successful principals.
- Ability names are validated; duplicate definitions fail; undefined abilities deny access.
- Gates handle broad abilities and policies accept typed resources.
- Cookie sessions, CSRF, OAuth/OIDC, JWT, MFA, password reset, and rate limiting are not silently enabled.

## API resources and OpenAPI

- Database models are not automatically serialized as public API responses.
- Single resources use a `data` envelope; collections support explicit links and metadata.
- OpenAPI documents declare an API title, version, paths, operations, reusable schemas, and security schemes.
- Duplicate component, operation, operation-ID, parameter, property, request-body, and response definitions fail during setup.
- Every path-template segment has one matching required path parameter in declaration order.
- Every operation defines at least one response.
- Invalid response statuses and malformed component names fail before export.
- Unresolved schema and security references fail before JSON is produced.
- Documented route registration updates the runtime router and document together or leaves the document unchanged.
- Generated OpenAPI uses version 3.1.0 and valid JSON.

## Operational features

- Built-in request logs exclude query strings, headers, bodies, tokens, and cookies.
- Request events include method, path, status, duration, and available request/trace correlation IDs.
- Metrics use fixed names without user-controlled labels or unbounded cardinality.
- Active-request accounting returns to zero after success and error paths.
- Liveness remains independent of external dependencies; readiness returns `503` for unhealthy checks.
- Readiness output exposes check state without internal diagnostic details.
- W3C trace version `00`, length, hexadecimal, and nonzero-ID rules are enforced.
- Incoming trace IDs are preserved while a new server parent/span ID is generated.
- Rate-limit configuration rejects zero limits, subsecond windows, and zero key capacity.
- Rate-limit state has explicit key and key-length bounds and fails closed at capacity.
- Rejected requests return `429`, retry timing, and remaining-limit metadata.
- Proxy-derived client identity is never trusted implicitly.

## Cache and storage

- Cache keys and values have explicit bounds; expired values are removed before reads and capacity decisions.
- Cache `add` and signed-integer `increment` are atomic within a backend instance.
- Cache namespaces isolate identical logical keys without hiding the physical prefix.
- The memory cache evicts the least recently used entry when its entry capacity is reached.
- Storage paths are normalized relative paths and reject absolute paths, traversal, empty segments, backslashes, and control bytes.
- Memory and local storage enforce object-size limits before replacing an existing object.
- Local writes use same-directory temporary files and preserve the prior object if streaming or replacement fails.
- Local reads and listings reject symbolic links and non-regular objects.
- Listing is bounded and deterministic; a prefix matches a complete object or path subtree, not a textual sibling prefix.
- Remote cache/storage adapters remain optional and can construct the public component errors.

## Events and background jobs

- Event listeners are typed, run synchronously in registration order, and can be explicitly removed.
- Dispatch stops at the first listener error and reports the event and listener identity.
- Listener panics are contained when panic unwinding is enabled.
- Job queue capacity and worker count are validated and remain fixed after startup.
- Dispatch fails promptly when the bounded queue is full or closed.
- Job attempts are numbered, retries use bounded backoff, and exhaustion creates a failed-job record.
- Handler panics are contained and participate in the same retry and failure rules as returned errors.
- Shutdown closes dispatch, drains accepted work and retries, and joins every worker.
- Queue snapshots expose pending, active, accepted, successful, failed, retry, panic, and failed-record counts without job-controlled labels.
- Failed-job memory storage is bounded and never stores the original job payload.
- A recurring schedule emits at most one job per schedule per tick and does not create an unbounded catch-up burst.
- Process-local queues explicitly make no durable, distributed, exactly-once, or crash-recovery guarantee.

## External communication

- URLs reject unsupported schemes, user information, fragments, invalid ports, control bytes, and ambiguous IPv6 authorities.
- Request header count, header bytes, and body bytes are bounded before connection work.
- Connect, read, and write operations use explicit nonzero timeouts.
- The built-in TCP client rejects HTTPS and never silently downgrades it to plaintext.
- Transport-owned Host, Connection, Content-Length, and Transfer-Encoding headers cannot be overridden.
- Response status, headers, content length, transfer encoding, chunk framing, trailers, and body size are validated within configured limits.
- Redirects are returned to the caller and are never followed automatically.
- Request and response debug output excludes header values and body contents; URL paths and queries are redacted.
- Email addresses and subjects reject control-byte injection; notification bodies have configurable limits.
- Webhooks require HTTPS by default and support an explicit idempotency key.
- Delivery reports preserve the result and attempt count for each requested channel.
- Retryable network failures, `408`, `429`, and `5xx` may retry; ordinary webhook `4xx` responses do not.
- Missing mail or HTTP transports are reported per channel rather than silently dropping delivery.
- In-memory mail transport is bounded and intended for tests, not production delivery.

## Developer tooling

- CLI parsing supports `new`, `make:model`, `make:migration`, `migrate`, `migrate:rollback`, and `migrate:status` without hidden folder discovery.
- Unknown, missing, and excess CLI arguments return usage errors rather than guessing intent.
- Project paths are normalized, relative, inside the selected root, and have an existing trusted parent.
- Project, model, module, and migration files are created without overwriting existing files.
- Partial project generation removes only the newly created target; model registration failure removes the newly created model.
- Package, PascalCase model, and snake_case migration names are validated before filesystem mutation.
- Generated migrations implement the actual driver-aware `Migration` statement contract.
- Migration commands delegate to an application-provided executor with a configured connection and registry.
- In-memory request tests execute the real router and middleware without opening TCP sockets.
- Fluent assertions cover status, success, headers, raw bodies, UTF-8 text, and structural JSON.
- Fake outbound HTTP preserves queued response order and records owned requests.
- Event and job recorders expose captured events and retry attempt numbers.
- Temporary test directories use collision-resistant names and recursively clean only their exact directory.
- The testing crate remains outside the production framework dependency graph.

## Reference application

- The foundation workspace member runs its schema through `MigrationRunner` rather than ad-hoc table creation.
- Its user controller implements the same `CrudController` contract emitted by CRUD code generation and registers through atomic `route.crud`.
- FormRequest sanitization, request-aware authorization, semantic validation, and database-aware uniqueness checks execute on real application routes.
- Configured bearer authentication distinguishes missing credentials from authenticated-but-forbidden requests.
- Route ability authorization and typed resource-policy authorization are both exercised.
- The user model eager-loads `has_many`, `has_one`, and `belongs_to_many` relationships through the real SQLite schema.
- Axe renders eager-loaded relationship data from the application's validated view tree.
- `berserk-testing::TestClient` covers create/show/update/delete, validation, relationship presentation, authentication, authorization, named CRUD routes, and the Axe route without opening a TCP socket.

## Hardening and maintenance

- Every crate forbids unsafe code and the workspace declares Rust 1.88 as its MSRV.
- Default diagnostics do not expose HTTP header values, query strings, path parameters, bodies, SQL, database bindings, cache keys, or cached values.
- Stable and MSRV CI definitions cover formatting, compilation, Clippy, tests, documentation, all features, and the independent consumer.
- Dependency advisory, license, duplicate-version, and source policies are automated and every exception requires review.
- Security reporting, support, compatibility, contribution, changelog, and release ownership are documented.
- Release preparation checks feature combinations, live databases, fuzzing, load/soak behavior, package contents, and clean-machine examples.
- The project cannot be marked release-ready until the executable gates pass, a license and private reporting channel exist, and the owner approves the artifacts.

## Phase 1 review items resolved in this document set

Explicit supported status range, raw path behavior, header text subset requirement, deferred status validation, and shutdown limits are documented. Exact network defaults and detailed parser grammar belong to their implementation phases and remain open.
