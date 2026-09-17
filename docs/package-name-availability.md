# crates.io package-name availability check

Date checked: 2026-09-17

This record covers the package identities prepared for Berserk `0.1.0`.

## Method

The publishable package set was taken from the repository package workflow and workspace metadata expectations. Each intended package name was then checked against the official crates.io package index (`rust-lang/crates.io-index`) using the canonical sparse-index path for that package name.

At the time of this check, every canonical index lookup returned no package entry.

## Results

| Package | Result on 2026-09-17 |
| --- | --- |
| `berserk` | No crates.io index entry found |
| `berserk-core` | No crates.io index entry found |
| `berserk-validation` | No crates.io index entry found |
| `berserk-database` | No crates.io index entry found |
| `claw-orm` | No crates.io index entry found |
| `berserk-auth` | No crates.io index entry found |
| `berserk-openapi` | No crates.io index entry found |
| `berserk-cache` | No crates.io index entry found |
| `berserk-storage` | No crates.io index entry found |
| `berserk-events` | No crates.io index entry found |
| `berserk-jobs` | No crates.io index entry found |
| `berserk-client` | No crates.io index entry found |
| `berserk-notifications` | No crates.io index entry found |
| `berserk-cli` | No crates.io index entry found |
| `berserk-testing` | No crates.io index entry found |

## Release note

crates.io package names are allocated on a first-come, first-served basis. This check confirms that no collision was present when the release gate was reviewed; it does not reserve the names. Re-run the name check immediately before the first publication, and stop publication if any intended name has been claimed in the meantime.

No crate was published, reserved, tagged, or otherwise released by this check.
