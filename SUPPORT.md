# Support

Berserk v1.0.0 is the first public stable release.

The support matrix below defines the supported v1.0.0 contract:

- Minimum supported Rust version (MSRV): **Rust 1.88**.
- Stable Rust is the primary development toolchain.
- Linux x86_64 is the primary release-validation platform.
- Windows and macOS receive development compatibility coverage.
- PostgreSQL, MySQL, and SQLite support is feature-gated and defined by the
  tested matrix.
- Users should normally upgrade to the latest compatible Berserk release to receive fixes.
- After v1.0.0, breaking public API changes require a new major release;
  security and correctness fixes should remain within semantic-versioning
  constraints whenever practical.

The authoritative platform, toolchain, and database matrix is maintained in
[docs/support-policy.md](docs/support-policy.md). Release readiness is tracked in
[docs/v1-maturity-gate.md](docs/v1-maturity-gate.md), and security reporting is
covered by [SECURITY.md](SECURITY.md).
