# Berserk v1 maturity gate

Berserk v1.0.0 is the first public stable release, not a claim that the project
has Laravel's age, package ecosystem, community size, or years of production
history. The release target is a mature core: coherent APIs, predictable
behavior, strong tooling, clear extension boundaries, and enough end-to-end
coverage that application developers are not forced to assemble framework
plumbing manually.

Do not publish v1.0.0 merely because the package version is already set to
1.0.0. Publish only after every blocker below is complete on the exact release
candidate.

## Framework maturity blockers

- [x] Centralize generated Rust source in `berserk-codegen`; the CLI must not
  maintain an independent Rust template system.
- [ ] Introduce a shared module/application specification that can coordinate
  model, request, controller, migration, and route generation without
  duplicating names or conventions.
- [ ] Provide one coherent CRUD generation workflow and compile its complete
  output as an independent consumer application.
- [ ] Preserve normal Claw query composition after named eager loading so
  filtering, ordering, limits, and pagination do not become a separate query
  experience after `.with(...)`.
- [ ] Support nested named eager loading with explicit, bounded semantics such
  as `posts.comments` and `roles.permissions`.
- [ ] Give `LoadedPage<M, NamedRelations>` first-class JSON and Axe
  presentation, including pagination metadata.
- [ ] Bound or chunk eager-load key queries so large parent collections do not
  fail solely because a database driver has a parameter limit.
- [ ] Complete Axe's production template path with deterministic precompilation
  or an equivalent build-time validation step, useful file/line diagnostics,
  and verified include/dependency behavior.
- [ ] Maintain at least one realistic reference application that exercises
  generated CRUD, validation, migrations, relationships, authentication,
  authorization, views, and application tests together.
- [ ] Perform an independent API/security review and resolve or explicitly
  document every release-blocking finding.

## Release-candidate blockers

After the framework maturity blockers are complete:

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

## What is not a v1 blocker

A custom Berserk source language/transpiler is not required for v1. It should
be built only after the Rust-facing contracts and shared application
specification are stable. Likewise, feature-count parity with Laravel is not a
release criterion; copying features without a demonstrated framework need would
make the core less coherent rather than more mature.
