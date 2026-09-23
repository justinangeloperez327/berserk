# Berserk v1 maturity gate

Berserk v1.0.0 is intended to be the first public stable release, not a claim that the project
has Laravel's age, package ecosystem, community size, or years of production
history. The release target is a mature core: coherent APIs, predictable
behavior, strong tooling, clear extension boundaries, and enough end-to-end
coverage that application developers are not forced to assemble framework
plumbing manually.

Do not publish v1.0.0 merely because the package version is already set to
1.0.0. Publish only after every blocker below is complete on the exact release
candidate.

## Current status

The implementation-side V1 core is mature enough for the first stable release
candidate. The owner has explicitly chosen to publish v1.0.0 without completing
a pre-release independent API/security review. That decision does not count as
an audit and does not permit Berserk to be described as independently audited or
security-certified.

Normal `main` CI passing is necessary but does not satisfy the final
release-candidate gates, because those checks must be rerun against the exact
source commit that would be published.

## Framework maturity blockers

- [x] Centralize generated Rust source in `berserk-codegen`; the CLI must not
  maintain an independent Rust template system.
- [x] Introduce a shared module/application specification that can coordinate
  model, request, controller, migration, and route generation without
  duplicating names or conventions.
- [x] Provide one coherent CRUD generation workflow and compile its complete
  output as an independent consumer application.
- [x] Preserve normal Claw query composition after named eager loading so
  filtering, ordering, limits, and pagination do not become a separate query
  experience after `.with(...)`.
- [x] Support nested named eager loading with explicit, bounded semantics such
  as `posts.comments` and `roles.permissions`.
- [x] Give `LoadedPage<M, NamedRelations>` first-class JSON and Axe
  presentation, including pagination metadata.
- [x] Bound or chunk eager-load key queries so large parent collections do not
  fail solely because a database driver has a parameter limit.
- [x] Complete Axe's production template path with deterministic precompilation
  or an equivalent build-time validation step, useful file/line diagnostics,
  and verified include/dependency behavior.
- [x] Maintain at least one realistic reference application that exercises
  generated CRUD, validation, migrations, relationships, authentication,
  authorization, views, and application tests together.
- [~] Independent API/security review deferred by explicit owner decision for
  v1.0.0. This is a documented risk acceptance, not completion of the review.
  A post-release independent review remains recommended.

## Release-candidate blockers

For the v1.0.0 publication candidate:

- [ ] Freeze the intended 1.0 public API and stop adding convenience features
  that are not required to correct a demonstrated usability or correctness gap.
- [ ] Rerun formatting, Clippy, tests, documentation, MSRV, feature matrices,
  live PostgreSQL/MySQL/SQLite checks, fuzzing, load/soak tests, package checks,
  and external-consumer verification against the exact release commit.
- [ ] Re-check package-name availability and the coordinated publication graph.
- [ ] Generate release checksums/provenance for the exact release commit.
- [ ] Review the final public docs, limitations, changelog, upgrade notes, and
  examples against that same commit.
- [ ] Obtain explicit owner approval for the exact commit and package set.

## Core maturity versus ecosystem maturity

Completing this gate means the Berserk **V1 framework core** is ready to enter
stable release engineering. It does not mean Berserk has Laravel's production
history, third-party package ecosystem, community size, support history, or
years of backward-compatibility evidence. Those are post-release outcomes; see
[ecosystem-maturity.md](ecosystem-maturity.md).

## What is not a v1 blocker

A custom Berserk source language/transpiler is not required for v1. It should
be built only after the Rust-facing contracts and shared application
specification are stable. Likewise, feature-count parity with Laravel is not a
release criterion; copying features without a demonstrated framework need would
make the core less coherent rather than more mature.
