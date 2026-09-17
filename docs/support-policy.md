# Berserk 0.1.0 support policy

This policy defines the support boundary intended for the first public `0.1.0` release. It is deliberately narrower than the set of systems where Berserk may happen to compile.

## Rust toolchain

- Minimum supported Rust version (MSRV): **Rust 1.88**.
- The current stable Rust toolchain is also tested.
- A future MSRV increase must be documented before release and reflected in workspace metadata and CI.

## Host operating systems

### Tier 1: production support

- **Linux x86_64**, validated on **Ubuntu 24.04 LTS**.

Tier 1 means the release-blocking quality, MSRV, live-database, security, fuzzing, load, and package workflows are expected to remain green for the release candidate. Production claims for `0.1.0` are limited to this tier.

### Tier 2: development compatibility

- **Windows**, validated on the current GitHub-hosted `windows-latest` runner.
- **macOS**, validated on the current GitHub-hosted `macos-latest` runner.

Tier 2 runs workspace compile/tests and the standalone minimal API example. These platforms are supported for development compatibility, but `0.1.0` does not make the same production-operability claim as Tier 1 because live database services, load/soak, and fuzzing are not executed there.

Other operating systems, Linux distributions, and architectures may work but are not part of the `0.1.0` support contract unless they are added to CI and this document.

## Database compatibility

Database support means Berserk's driver contract and migration tests run against a real server in CI for the listed server line.

| Database | Supported for `0.1.0` | CI evidence |
| --- | --- | --- |
| PostgreSQL | **15, 16, 17, 18** | Real server test for each major version using the current official major Docker image |
| MySQL | **8.4 LTS** | Real MySQL 8.4 server test |
| SQLite | **Bundled SQLite through `rusqlite 0.40.2`** | Driver/migration tests using the crate's `bundled` SQLite feature |

For PostgreSQL, use the latest available minor release in the supported major line. The CI major tags intentionally follow the current patch release in each major version.

PostgreSQL 14 and older are not part of the `0.1.0` support contract. PostgreSQL 14 reaches upstream end of support on 2026-11-12, so Berserk does not add a new release commitment to that line shortly before its retirement. PostgreSQL 19 prereleases are not supported.

MySQL versions other than 8.4 LTS, including MySQL 8.0 and 9.x/other later lines, are not claimed as supported until they are added to the live-database matrix. MariaDB is not claimed as MySQL-compatible for `0.1.0`.

The SQLite feature uses `rusqlite` with its `bundled` feature. Berserk therefore supports the SQLite library compiled by that dependency; compatibility with an arbitrary system-installed SQLite library is not part of the `0.1.0` contract.

## What CI enforces

- Ubuntu 24.04 runs the main workspace quality and MSRV gates.
- Windows and macOS run cross-platform workspace compile/tests plus the minimal API example.
- PostgreSQL 15 through 18 run the same live driver/migration contract tests.
- MySQL 8.4 runs the live driver/migration contract tests.
- SQLite runs its driver/migration contract tests with the bundled library.

A platform or database line should not be added to the public support table until an appropriate automated test exists. Likewise, removing a supported target requires an explicit support-policy update and release note.

## Upstream lifecycle references

- PostgreSQL versioning policy: https://www.postgresql.org/support/versioning/
- MySQL supported platforms: https://www.mysql.com/support/supportedplatforms/database.html
