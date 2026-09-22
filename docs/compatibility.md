# Compatibility and versioning

## Current status

Berserk v1.0.0 is the first public release target. Berserk has not yet been published publicly; completed repository work is being consolidated into this 1.0 baseline. Rust 1.88 is the minimum supported Rust version (MSRV). CI and release workflows define the tested feature, platform, and database combinations; the support matrix is documented in [support-policy.md](support-policy.md).

The intended 1.0 public API has been stabilized, but the **formal release freeze is not complete yet**. The independent API/security review is still open; after it closes, the exact 1.0 candidate is frozen and release validation is rerun against that commit. Once v1.0.0 is published, breaking public API changes require a new major release. New aliases and convenience surfaces should still be avoided unless they solve a concrete gap. Compatibility-affecting changes must be documented rather than introduced silently.

## Versioning policy

- Follow semantic versioning for published packages.
- Before 1.0, document breaking API or behavior changes in `CHANGELOG.md` and provide migration guidance when practical.
- Raising the MSRV is a compatibility change and must be reflected in release notes, workspace metadata, CI, and the support policy.
- Optional components remain feature-gated so applications can avoid subsystems they do not use.
- Supported feature combinations must compile independently and under the repository's all-feature quality gate.
- Database behavior that differs by backend remains explicit through capabilities and driver-specific tests.
- Serialized API, configuration, storage, and database formats require compatibility consideration independently of Rust type compatibility.
- Removing a supported platform or database line requires an explicit support-policy update and release note.

## Pre-1.0 policy

Minor releases may contain breaking changes while Berserk is below 1.0. Such changes should be deliberate, documented, and accompanied by migration guidance when practical.

For v0.9.x, the default direction is stabilization: prefer additive fixes, documentation corrections, and removal of ambiguity over new public concepts. Security, correctness, or unsoundness fixes may still require faster changes than the normal deprecation path.

## 1.0 candidate surface

The current coordinated publication graph contains 18 publishable crates. Package count is a release-engineering fact, not a reason to expose additional framework concepts.

The APIs intended to define the 1.0 application surface are:

- `App` plus `app.route()` for application assembly and routing;
- `Request` / `Response` and FormRequest-style typed input;
- explicit middleware and application state;
- request-scoped authentication and authorization;
- feature-gated database/Claw, services, testing, and operational components.

Compatibility helpers such as direct `App::get/post/...` registration and request-global auth facades are not the design center for 1.0. They may remain during the transition, but new application code should use the preferred scoped APIs.

`Arr` and `Str` remain explicit utilities and are intentionally excluded from the default prelude in v0.9.

## Deprecation

As the framework approaches 1.0, deprecated APIs should remain available long enough for a documented transition when practical. After 1.0, Berserk should use a more conservative compatibility and deprecation policy appropriate for a stable framework.

## Release ownership

Only the project owner decides when and where to publish. Automation may build, test, package, and verify release artifacts but must not publish by default.

## Relationship compatibility in 1.0

Existing relationship constructors and HasOne/RelatedSet result types are preserved. The explicit `load(connection, ...)` methods remain as deprecated forwarders to `load_on`. New scoped loading is available through `Relationship::load`. Integer relationship keys now compare equal across signed/unsigned driver representations; malformed NULL pivot keys return Decode errors instead of disappearing silently. See [relationships](relationships.md) and [upgrade notes](upgrade-notes.md).
