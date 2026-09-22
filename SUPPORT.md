# Support

Berserk v1.0.0 is the intended first public stable release. It has **not** yet
been published, so there is no published stable Berserk version to support at
this time.

The support matrix below defines the target contract for the first public
release candidate:

- Minimum supported Rust version (MSRV): **Rust 1.88**.
- Stable Rust is the primary development toolchain.
- Linux x86_64 is the primary release-validation platform.
- Windows and macOS receive development compatibility coverage.
- PostgreSQL, MySQL, and SQLite support is feature-gated and defined by the
  tested matrix.
- Once v1.0.0 is published, users should normally upgrade to the latest
  compatible release to receive fixes.
- After v1.0.0, breaking public API changes require a new major release;
  security and correctness fixes should remain within semantic-versioning
  constraints whenever practical.

The authoritative platform, toolchain, and database matrix is maintained in
[docs/support-policy.md](docs/support-policy.md). Release readiness is tracked in
[docs/v1-maturity-gate.md](docs/v1-maturity-gate.md), and security reporting is
covered by [SECURITY.md](SECURITY.md).
