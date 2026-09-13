# Phase 22 — Cache and Storage

Phase 22 introduces two optional, HTTP-independent crates. Both APIs favor small explicit contracts so remote adapters can be added without making network services part of a default build.

## Cache

`framework-cache` stores raw bytes under validated keys. `Cache` exposes `get`, `put`, atomic `add`, `forget`, atomic signed-integer `increment`, and a convenience `remember` operation. Terminal calls are visible; serialization remains the application's or a future typed wrapper's responsibility.

`MemoryCache` has explicit entry and value-size limits, millisecond expiration, expired-entry cleanup, and least-recently-used eviction. `Namespaced<C>` prefixes physical keys so features can share a backend without accidental collisions. `remember` intentionally does not promise single-flight loading: concurrent misses may run the loader more than once.

## Storage

`framework-storage` addresses objects through `StoragePath`, a normalized relative path. `Storage` exposes streaming writes and reads, metadata, deletion, bounded listing, and existence checks. `MemoryStorage` supports tests and small ephemeral uses. `LocalStorage` creates nested folders under a canonical root and uses bounded same-directory temporary writes before replacement.

Local paths reject traversal, absolute paths, backslashes, empty path segments, control bytes, symbolic links, and non-regular objects. This protects normal application use, but portable standard-library path checks cannot eliminate filesystem time-of-check/time-of-use races. The configured root and its ancestors must not be writable by an untrusted local process.

Listings are sorted and capped. Prefixes select a complete object or a directory subtree. Object writes replace an existing object only after the new content has been fully read within its byte limit.

## Feature selection

The main package keeps both integrations disabled by default:

```toml
framework = { path = "crates/framework", features = ["cache", "storage"] }
```

Applications can also depend directly on `framework-cache` or `framework-storage`. Future Redis, Memcached, S3-compatible, or cloud-provider adapters must be opt-in and preserve the shared contracts; Phase 22 does not choose or ship those dependencies.

## Verification status

Contract tests cover cache lifecycle, expiration, eviction, namespaces, concurrent increments, storage path validation, memory object lifecycle, bounded writes, local nesting, and replacement. Rust tooling is unavailable in the preparation environment, so compilation and test execution remain pending.
