# Known limitations

This document records intentional or unresolved boundaries of the Berserk v1.0.0 public release target. They are not necessarily defects, but applications must account for them.

## Security review status

Berserk is not independently security-certified. Internal review, automated security checks, fuzzing, and other repository gates do not replace an independent security assessment.

## TLS termination

The built-in HTTP server does not terminate TLS. Internet-facing deployments require HTTPS termination through a trusted reverse proxy or a vetted TLS adapter.

## Outbound HTTPS and SSRF policy

The built-in TCP HTTP transport does not provide a complete application SSRF policy. Applications accepting user-influenced destinations must enforce allowed schemes, hosts, ports, and network ranges before dispatching requests. HTTPS requires an appropriate TLS-capable transport.

## Synchronous and async boundaries

Berserk remains sync-first. Optional async actions use the framework's async adapters, while synchronous database drivers remain blocking. Rust cannot safely force-stop arbitrary user code that blocks forever.

Spawned tasks do not automatically inherit request-local database or authentication scope. Long-running work should use an appropriate job or external-worker design.

## Browser authentication policy

The current auth component provides authentication and authorization primitives, but applications using cookie-based credentials must define appropriate Secure, HttpOnly, SameSite, rotation/revocation, and CSRF behavior. The 1.0 target includes strengthened bearer authentication and HTTP security, but browser cookie policy remains application-defined.

## Memory session store

`MemorySessionStore` is process-local and unbounded. It is appropriate for development and controlled workloads, not as a general persistent production session store.

## Local storage trust boundary

`LocalStorage` validates normalized relative paths, rejects traversal and observed symlinks, bounds operations, and uses safe write behavior where documented. Its standard-library implementation assumes the storage tree is not concurrently rewritten by an untrusted local actor.

## Request transactions

Request-scoped transactions reuse the request connection. Nested request transactions are not currently supported. Backend capabilities still apply.

## Database and ORM boundaries

Supported database versions are defined in [support-policy.md](support-policy.md). Synchronous SQL drivers remain blocking. Claw splits eager-load key lookups into bounded batches, but very large eager-loaded result sets can still consume substantial memory; paginate parent queries for response-size and memory control. Offset pagination requires explicit ordering and a transaction when count/items must share a snapshot. Some model behaviors, such as timestamps and soft deletes, remain application responsibilities.

## Platform support

Linux x86_64 is the strongest release-validation target. Windows and macOS receive development compatibility coverage but not necessarily every live-database, fuzz, load, or soak workflow used on Linux.

## Feature combinations

Berserk is modular. Applications must enable the features required by the APIs they use. The `server` feature controls network serving; in-memory application handling can be used without it. The `async` feature is independently optional.

## Stable compatibility

v1.0.0 will establish the stable public API baseline when published. Breaking public API changes require a new major release under semantic versioning. Compatibility-affecting changes should be recorded in `CHANGELOG.md` and [upgrade-notes.md](upgrade-notes.md).

## Relationship boundaries

`sync` and `attach_many` use one transaction and reject nesting; they do not serialize competing writers. Eager loads chunk large key lists into conservative bounded batches, but they still materialize the requested related models in memory. Paginate parents for memory and response-size control. Many-to-many queries preserve pivot rows and require qualified ambiguous columns. Joined mutations are unsupported. HasOne retains RelatedSet for compatibility. Foreign/owner key accessors remain explicit. See [relationships](relationships.md).
