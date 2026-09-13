# Support policy

The project has no supported release yet. Source snapshots are development artifacts, not stability promises.

The intended initial policy is:

- Rust 1.82 is the minimum supported Rust version (MSRV).
- Stable Rust is the primary toolchain.
- PostgreSQL, MySQL, and SQLite are optional and tested independently as well as together.
- The latest minor release receives fixes before 1.0; older pre-1.0 minors may require upgrading.
- Security fixes may require breaking changes before 1.0.

This policy becomes binding only after CI passes, a license and final package names are selected, and the first release is explicitly published by the owner.
