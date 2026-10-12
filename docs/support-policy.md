# Berserk support policy

This guide describes the operating systems, Rust toolchain, and databases covered by Berserk's published support baseline. Other configurations may compile, but are not necessarily tested.

## Rust toolchain

- Minimum supported Rust version (MSRV): **Rust 1.88**.
- The current stable Rust toolchain is also tested.

## Host operating systems

### Tier 1: release support

- **Linux x86_64**, validated on **Ubuntu 24.04 LTS**.

Tier 1 receives the broadest automated validation, including live database, security, load, and packaging checks. Automated testing does not imply independent security certification.

### Tier 2: development compatibility

- **Windows**, validated on the current GitHub-hosted `windows-latest` runner.
- **macOS**, validated on the current GitHub-hosted `macos-latest` runner.

Tier 2 receives compile and test coverage with all features. It does not receive the full Linux live-database, fuzzing, packaging, and load coverage.

The standalone minimal API and fresh external-consumer checks run on the Tier-1 Linux CI path rather than being duplicated on every Tier-2 runner.

Other operating systems, Linux distributions, and architectures may work but are not part of the current support contract unless they are added to CI and this document.

## Database compatibility

Database support means Berserk's driver contract and migration tests run against a real server in CI for the listed server line.

| Database | Version or integration | Automated coverage |
| --- | --- | --- |
| PostgreSQL | **18 (latest patch line)** | Real PostgreSQL 18 server test using the current official major Docker image |
| MySQL | **26.7 (latest production Innovation line)** | Real MySQL 26.7 server test |
| SQLite | **Latest bundled SQLite through the current `rusqlite` dependency** | Driver/migration tests using the crate's `bundled` SQLite feature |
| MongoDB | **8.3.11 server baseline** | Real MongoDB 8.3.11 service/version smoke test; framework driver support is not yet claimed |

The listed server versions are the integration-test baseline, not a claim of compatibility with every earlier or prerelease version.

The SQLite feature uses `rusqlite` with its `bundled` feature, so SQLite is upgraded through the Rust dependency rather than a separate server image. Compatibility with an arbitrary system-installed SQLite library is not part of the v1.0.0 contract.

MongoDB is intentionally listed as a server baseline only. Berserk does not yet expose a MongoDB `Connection` implementation, query dialect, or Claw persistence adapter, so the repository must not claim MongoDB application-level support until those contracts are designed and tested.

## What CI enforces

- Ubuntu 24.04 runs the main workspace quality and MSRV gates.
- Windows and macOS run cross-platform workspace compile/tests with all features.
- The standalone minimal API and fresh external-consumer builds run on Ubuntu.
- PostgreSQL 18 runs the live driver/migration contract tests.
- MySQL 26.7 runs the live driver/migration contract tests.
- SQLite runs its driver/migration contract tests with the bundled library.
- MongoDB 8.3.11 runs a live server/version smoke test; this does not imply framework-driver support.

The support table reflects currently tested integrations. For deployments on other versions, run your own compatibility tests.

## Upstream lifecycle references

- PostgreSQL versioning policy: https://www.postgresql.org/support/versioning/
- MySQL supported platforms: https://www.mysql.com/support/supportedplatforms/database.html
