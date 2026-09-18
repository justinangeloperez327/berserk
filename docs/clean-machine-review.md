# Clean-machine release review

Date: 2026-09-17

This historical review checked an earlier Berserk candidate from the perspective of a new consumer rather than from inside the workspace implementation. It records evidence from 2026-09-17 and must not be treated as validation of a later release candidate.

## Scope

Reviewed against the current candidate:

- `README.md` installation and quick-start instructions;
- the standalone `examples/minimal-api` consumer workspace;
- the `berserk` facade package metadata and feature names;
- `docs/public-api.md` against the facade exports and implemented routing surface;
- the release package workflow and package-file inspection checks.

## Findings

No blocking drift was found between the README quick-start surface and the current facade exports. The documented `App`, `Response`, `Result`, typed route parameter, and optional feature names remain present in the `berserk` facade.

The existing `examples/minimal-api` project is already excluded from the workspace and therefore provides useful independent-consumer coverage, but it still resides inside the repository. To make the release check stronger, the Package workflow now creates fresh temporary Cargo projects outside the repository workspace and compiles them without a Rust build cache.

The clean-consumer job checks two cases:

1. the README quick-start API using the default `berserk` feature set;
2. an external consumer using the documented optional feature combination `sqlite`, `claw`, `auth`, and `openapi`.

The normal CI quality job continues to run the richer `examples/minimal-api` HTTP contract tests.

## Release interpretation

This is a clean consumer/build review, not an independent security audit and not a crates.io publication test. Before publication, package-name availability must still be rechecked and the separate independent security/API review gate remains open.

The clean-machine release-checklist item should be considered satisfied only when the Package workflow's `clean-consumer` job passes for this change.
