# Known limitations

These are the operational boundaries of Berserk v1.0.0 that applications need to account for.

## Security review status

Berserk is not independently security-certified. Internal review, automated security checks, fuzzing, and other repository gates do not replace an independent security assessment.

## TLS termination

The built-in HTTP server does not terminate TLS. Internet-facing deployments require HTTPS termination through a trusted reverse proxy or a vetted TLS adapter.

## Outbound HTTPS and SSRF policy

The built-in TCP HTTP transport does not provide a complete application SSRF policy. Applications accepting user-influenced destinations must enforce allowed schemes, hosts, ports, and network ranges before dispatching requests. HTTPS requires an appropriate TLS-capable transport.

## Synchronous and async boundaries

Berserk remains sync-first. Optional async actions use the framework's async adapters, while synchronous database drivers remain blocking. Rust cannot safely force-stop arbitrary user code that blocks forever.

Spawned tasks do not automatically inherit request-local database or authentication scope. Long-running work should use an appropriate job or external-worker design.

Scheduler ticks dispatch a snapshot of due entries. Factories run outside the scheduler state lock and may register or forget schedules for future ticks. Concurrent or recursive ticks fail immediately; already selected entries may still dispatch after removal. Worker shutdown joins every worker before reporting a worker panic.

## Browser authentication policy

The current auth component provides authentication and authorization primitives, but applications using cookie-based credentials must define appropriate Secure, HttpOnly, SameSite, rotation/revocation, and CSRF behavior. Bearer authentication and HTTP security helpers are available, but browser cookie policy remains application-defined.

## Memory session store

`MemorySessionStore` is process-local and capacity-bounded. The default limit is 10,000 records and `MemorySessionStore::new` accepts an explicit positive capacity. It is appropriate for development and controlled workloads, not as a persistent or distributed production session store.

## Local storage trust boundary

`StoragePath` uses portable relative names: Windows drive prefixes, alternate data streams, device names, forbidden filename characters, and trailing dots/spaces are rejected on every platform. `LocalStorage` rejects traversal and observed symlinks, bounds operations, and uses safe write behavior where documented. Its standard-library implementation assumes the storage tree is not concurrently rewritten by an untrusted local actor.

## Axe view deployment

Release builds cache the compiled Axe view tree after its first use. Editing
template files underneath a running release process does not invalidate that
cache; deploy validated templates together with the application and restart the
process when view files change. Debug builds recompile the view tree so local
template edits are visible. Include targets are static root-relative view names;
dynamic runtime include paths are intentionally unsupported.

## Request transactions

Request-scoped transactions reuse the request connection. Nested request transactions are not currently supported. Backend capabilities still apply.

## Database and ORM boundaries

Supported database versions are defined in [support-policy.md](support-policy.md). Synchronous SQL drivers remain blocking. Claw splits eager-load key lookups into bounded batches, but very large eager-loaded result sets can still consume substantial memory; paginate parent queries for response-size and memory control. Offset pagination requires explicit ordering and a transaction when count/items must share a snapshot. Some model behaviors, such as timestamps and soft deletes, remain application responsibilities.

The PostgreSQL driver maps boolean, integer/OID, floating-point, text, and binary scalar values. Native date/time, decimal, JSON, UUID, array, and custom column types require an explicit supported SQL cast or an application adapter; declaring a column through the migration DSL does not add a value codec.

SQLite generated rebuilds reject referenced tables while foreign-key enforcement is enabled, before changing schema or child rows. Use a verified manual procedure for those changes; see [migrations.md](migrations.md).

## Platform support

Linux x86_64 is the strongest release-validation target. Windows and macOS receive development compatibility coverage but not necessarily every live-database, fuzz, load, or soak workflow used on Linux.

## Feature combinations

Berserk is modular. Applications must enable the features required by the APIs they use. The `server` feature controls network serving; in-memory application handling can be used without it. The `async` feature is independently optional.

## Stable compatibility

v1.0.0 establishes the stable public API baseline. Breaking public API changes require a new major release under semantic versioning. Consult [compatibility](compatibility.md) and the [changelog](../CHANGELOG.md) when upgrading.

## Relationship boundaries

`sync` and `attach_many` use one transaction and reject nesting; they do not serialize competing writers. Eager loads chunk large key lists into conservative bounded batches, but they still materialize the requested related models in memory. Paginate parents for memory and response-size control. Many-to-many queries preserve pivot rows and require qualified ambiguous columns. Joined mutations are unsupported. HasOne retains RelatedSet for compatibility. Foreign/owner key accessors remain explicit. See [relationships](relationships.md).

## Fallible randomness

`TraceLayer` propagates operating-system entropy failures as framework errors.
Applications can use `TraceContext::try_generate`, `TraceContext::try_child`, and
`SessionToken::try_generate` for fallible generation. Their existing infallible
counterparts remain available for compatibility and may panic on entropy failure.
Collection randomization helpers also require working operating-system entropy.
