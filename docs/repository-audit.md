# Repository audit — October 2026

The audit started from main commit `e4385b3c1a56f7040a60ce0b886e2a4d72e0d22e`.
Its scope includes the tracked-file inventory, all 18 publishable crates,
the foundation application, the independent minimal example, generators,
compatibility fixtures, manifests, lockfiles, workflows, and documentation.
This is a repository maintenance review, not an independent security assessment.

## Repairs

| Area | Finding and change | Regression coverage |
| --- | --- | --- |
| SQLite migrations | Rebuilding referenced parents could trigger cascading child deletion or updates. Generated rebuilds now reject those tables before creating the replacement when foreign keys are enabled. | CASCADE, SET NULL, and NO ACTION preserve children and leave the migration unapplied; unreferenced rebuilds retain rows. |
| PostgreSQL | NULL parameters were rejected and numeric values used encoders with the wrong wire widths. Bindings now follow prepared statement types with checked integer narrowing and REAL conversion. | NULL encoders, numeric widths, overflow, incompatible types, and mandatory live-driver tests. |
| HTTP client | An unterminated header or chunk line could allocate past the configured limit before validation. Reads now consume at most the remaining budget plus one detection byte. | Oversized unterminated lines and exact-budget CRLF lines. |
| JSON | Every ordinary string character revalidated the remaining UTF-8 suffix. Input is validated once and parsing advances through that validated string. | Large mixed ASCII/Unicode input with an escaped suffix, alongside existing malformed-input tests. |
| Cache | Increment could store a counter larger than its value-size cap. The encoded result is checked before mutation or eviction. | Overflowing counter strings preserve existing values and entries. |
| Rate limits | Each request scanned all tracked keys. Existing keys now reset their own window; expired-key cleanup occurs when a new key needs capacity. | Window expiry, active limits, capacity reclamation, and full-capacity rejection. |
| Scheduler | Sub-millisecond intervals became zero when converted to scheduler timestamps. Intervals below one millisecond now fail configuration validation. | Rejection of sub-millisecond intervals and existing scheduling/retry tests. |
| Foundation | Stale migration, relationship, response, service, and failed-job-store calls prevented compilation or successful integrations. Calls now use the current public contracts. | CRUD, nested eager loading, views, read-only denial, and authorized integration dispatch. |
| Authorization | A read ability allowed dispatching background work. Integration now requires `tasks.integrate`. Events and memory notifications describe snapshot dispatch. | A project reader receives 403; an authorized operator receives 200. |
| Fixtures and source | Transaction fixtures used the connection interface, Claw factories double-boxed connections, and assertions contradicted current error and routing contracts. The obsolete uncompiled synchronous transport was removed. | Workspace tests, Claw live relationships, and all-target compilation. |
| External API | The existing v1 consumer fixture required a missing `response().bytes(...)` method and incorrectly declared borrowed keys for Claw's established owned-key write traits. The binary terminal is implemented, the fixture type is corrected against source history, and the HTTP example is compiled externally. | Binary metadata/error validation and four independent compatibility consumers; the existing Claw traits are preserved. |

## Dependency and workflow consistency

External requirements now share the workspace dependency table. Compatible
upgrades include Hyper 1.12, hyper-util 0.1.21, MySQL 28.0.3, serde_json 1.0.154,
syn 3.0.8, Tokio 1.53.2, and zeroize 1.9.1, plus compatible transitive updates.
The unused direct rand_core dependency was removed. The independent example's
lockfile now references the actual 1.0.0 workspace packages.

Cargo resolver 3 prefers versions compatible with the declared Rust minimum.
UUID 1.26.1 is retained because the newer 1.27/1.28 versions require Rust 1.89.
Rust 1.88 and edition 2021 remain supported. Generated applications and external
fixtures use the same resolver.

Feature, minimum-Rust, platform, and database jobs use committed lockfiles.
Dependabot covers the root workspace, independent example, and fuzz workspace.
Repository hygiene checks parse tracked Python and TOML, reject tracked build
output, and verify local documentation links. Live SQL URL names use the
`BERSERK_` prefix with legacy `FRAMEWORK_` aliases retained. Database versions,
migration boundaries, and the historical CI finding are documented accurately.

## Verification and remaining boundaries

The repair branch is validated with native Rust workspace tests, all-feature
Clippy with warnings denied, Rust 1.88 compilation, documentation checks,
external generated consumers, and the GitHub workflow matrix. Exact workflow
results belong to the final PR commit's checks; local PostgreSQL/MySQL tests
require their dedicated URLs, while CI requires both live contracts.

Referenced SQLite rebuilds require a verified manual procedure; automatic
foreign-key-safe reconstruction is not implemented. PostgreSQL's scalar codecs
do not cover every type supported by the migration DSL. MongoDB receives a
server-version smoke check and has no driver. Blocking handlers cannot be
forcibly stopped, and memory services remain process-local. These boundaries
are recorded in [known-limitations.md](known-limitations.md).

Main branch protection was absent at the audited baseline. Enforcing required
checks and review through repository rules remains a maintainer settings action.
No cross-framework benchmark was added by this audit.
