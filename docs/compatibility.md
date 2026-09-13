# Compatibility and versioning

## Current status

The workspace is version `0.0.0`, unpublished, and not API-stable. Rust 1.82 is the declared MSRV because the implementation uses standard-library APIs stabilized in that release.

## Proposed policy

- Follow semantic versioning after package names and public APIs are finalized.
- Before 1.0, document breaking changes in the changelog and prefer migration guidance over silent behavior changes.
- Raising MSRV is a compatibility change and must be called out in release notes.
- Optional features are additive: default builds do not pull database, auth, queue, notification, or tooling components.
- Feature combinations must compile independently and under `--all-features`.
- Common database contracts have shared tests; backend-specific behavior stays explicit through capabilities and driver tests.
- Serialized API and database formats need their own compatibility notes when introduced; Rust type compatibility alone is insufficient.

## Deprecation

After the first public release, deprecated APIs should remain for at least one minor release when a safe transition is practical. Security or correctness issues may require faster removal, with an explicit advisory and migration path.

## Release ownership

Only the project owner decides when and where to publish. Automation may build, test, and package artifacts but must not publish by default.
