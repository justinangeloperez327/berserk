# crates.io package-name availability check

Date checked: 2026-09-17

This record covers the package identities prepared for Berserk `0.1.0`.

## Method

The publishable package set was taken from the repository package workflow and workspace metadata expectations. Each intended package name was checked against the official crates.io package index (`rust-lang/crates.io-index`) using its canonical index path.

Because crates.io performs case-insensitive collision detection and treats `-` and `_` as equivalent for package-name collisions, the check also covered every underscore-equivalent form (for example `berserk-core` / `berserk_core` and `claw-orm` / `claw_orm`). A repository search for the `berserk` family and the Claw normalized form was also used as a secondary check.

At the time of this review, no checked canonical or normalization-equivalent name had an index entry.

## Results

| Package | Result on 2026-09-17 |
| --- | --- |
| `berserk` | No crates.io index collision found |
| `berserk-core` | No crates.io index collision found |
| `berserk-validation` | No crates.io index collision found |
| `berserk-database` | No crates.io index collision found |
| `claw-orm` | No crates.io index collision found |
| `berserk-auth` | No crates.io index collision found |
| `berserk-openapi` | No crates.io index collision found |
| `berserk-cache` | No crates.io index collision found |
| `berserk-storage` | No crates.io index collision found |
| `berserk-events` | No crates.io index collision found |
| `berserk-jobs` | No crates.io index collision found |
| `berserk-client` | No crates.io index collision found |
| `berserk-notifications` | No crates.io index collision found |
| `berserk-cli` | No crates.io index collision found |
| `berserk-testing` | No crates.io index collision found |

## Release note

crates.io package names are allocated on a first-come, first-served basis. This check confirms that no collision was present when the release gate was reviewed; it does not reserve the names. Re-run the name and normalization-equivalent checks immediately before the first publication, and stop publication if any intended name has been claimed in the meantime.

No crate was published, reserved, tagged, or otherwise released by this check.
