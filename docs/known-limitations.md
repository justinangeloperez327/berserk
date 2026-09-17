# Known limitations

This document records intentional or currently unresolved boundaries for the Berserk `0.2.0` release candidate. These are not necessarily defects, but applications must account for them explicitly.

## Security review status

Berserk has completed an internal security/API review, but the release checklist still requires an independent external security/API review. The project has not received a third-party penetration test or security certification.

Do not describe the `0.2.0` candidate as production-certified.

## TLS termination

The built-in HTTP server does not terminate TLS. Internet-facing deployments require HTTPS termination through a trusted reverse proxy or a vetted TLS adapter.

## Outbound HTTPS and SSRF policy

The built-in `TcpHttpClient` transport intentionally sends plaintext `http` only. `https` requires a TLS-capable adapter.

The outbound client does not define an SSRF allowlist. Applications accepting user-influenced URLs must enforce allowed schemes, hosts, ports, and network ranges before dispatching requests.

## Synchronous request handlers

Berserk uses bounded Tokio/Hyper infrastructure internally while preserving synchronous public request handlers.

Rust cannot safely force-stop arbitrary synchronous application code. A handler that blocks forever can therefore prevent graceful shutdown from completing even though network I/O deadlines are bounded.

Applications should keep request-path work bounded and move long-running work to appropriate job or external worker infrastructure.

## Browser authentication policy

The auth component provides password, bearer-session, principal, guard, and authorization primitives, but it does not automatically define browser-cookie or CSRF policy.

Applications using cookies must define Secure, HttpOnly, SameSite, rotation/revocation, and CSRF behavior appropriate to their deployment.

## Memory session store

`MemorySessionStore` is process-local and unbounded. It is useful for development and controlled workloads, but deployments exposed to untrusted session creation should use a bounded and/or persistent store implementation with operational limits.

## Local storage trust boundary

`LocalStorage` validates normalized relative paths, rejects traversal and observed symlinks, bounds object/listing sizes, and uses temporary files for writes.

Its standard-library implementation assumes the storage tree is not concurrently rewritten by an untrusted local actor. Use an isolated storage root with appropriate operating-system permissions.

## Request transactions

Request-scoped transactions reuse the request connection and support commit/rollback behavior through the documented transaction API. Nested request transactions are not currently supported.

Driver capabilities still apply, and a backend may reject transaction options it does not support.

## Platform support

The strongest `0.2.0` host claim is Linux x86_64 validated on Ubuntu 24.04 LTS.

Windows and macOS receive development compatibility compile/test coverage, but they do not receive the same live-database, fuzzing, load, and soak validation as the Tier-1 Linux host.

Other operating systems, Linux distributions, and architectures are outside the documented support contract unless added to CI and `docs/support-policy.md`.

## Database support

The `0.2.0` support contract is intentionally narrow:

- PostgreSQL 15, 16, 17, and 18;
- MySQL 8.4 LTS;
- bundled SQLite through the supported `rusqlite` path.

PostgreSQL 14 and older, PostgreSQL 19 prereleases, MySQL versions outside 8.4 LTS, MariaDB, and arbitrary system-installed SQLite libraries are not claimed as supported.

## Optional components

No optional subsystem is enabled by default. Applications must explicitly select database drivers, Claw ORM, auth, OpenAPI, cache, storage, events, jobs, outbound client, notifications, and CLI support as needed.

This keeps the default dependency surface small but means examples that use optional APIs will not compile until the matching Cargo features are enabled.

## Pre-1.0 compatibility

`0.2.x` is pre-1.0. The API is documented and release-reviewed, but breaking changes may still occur in later pre-1.0 releases when necessary. Such changes should be recorded in `CHANGELOG.md` with migration guidance when practical.

## v0.2.0 scope and async boundaries

Synchronous SQL drivers remain blocking. Optional async actions occupy a blocking worker for their lifetime; disconnects, timeouts, and dropped response futures cannot force-stop started application code. Spawned tasks do not inherit database or principal scope. Transaction closures and explicit scope helpers are synchronous. Nested transactions are rejected. Eager loading is bulk and explicit, but very large collections can hit driver parameter limits. Offset pagination requires explicit ordering and a transaction when count/items must share a snapshot. Model timestamps and soft deletes require application code.
