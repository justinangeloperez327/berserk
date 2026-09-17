# Compatibility and versioning

## Current status

The workspace is prepared as the unpublished `0.1.0` release candidate. Rust 1.88 is the declared minimum supported Rust version (MSRV). CI checks the complete workspace on Rust 1.88 and on the current stable Rust toolchain, with platform and database coverage defined in `docs/support-policy.md`.

Berserk is still pre-1.0. Public APIs may change before a stable compatibility commitment is made, but release-candidate changes must be documented rather than introduced silently.

## Versioning policy

- Follow semantic versioning for published packages.
- Before 1.0, document breaking API or behavior changes in the changelog and provide migration guidance when practical.
- Raising the MSRV is a compatibility change and must be called out in release notes, workspace metadata, CI, and the support policy.
- Optional features are additive: default builds do not pull database, Claw ORM, auth, OpenAPI, cache, storage, events, jobs, outbound client, notifications, or CLI components.
- Feature combinations must compile independently and under `--all-features`.
- Common database contracts have shared tests; backend-specific behavior stays explicit through capabilities and driver tests.
- Serialized API and database formats require their own compatibility notes when introduced; Rust type compatibility alone is insufficient.
- Removing a supported platform or database line requires an explicit support-policy update and release note.

## Deprecation

After the first public release, deprecated APIs should remain available long enough for a documented transition when that is practical. Security, correctness, or unsoundness issues may require faster removal, in which case the release notes should explain the reason and migration path.

## Release ownership

Only the project owner decides when and where to publish. Automation may build, test, package, and verify release artifacts but must not publish by default.
