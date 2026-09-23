# Berserk roadmap

Berserk is preparing its first public v1.0.0 release. The framework core has
reached its intended V1 architectural scope; the remaining framework-maturity
blocker is the independent API/security review. After that review, work moves to
an exact-commit release-candidate process rather than another feature-expansion
cycle.

Earlier numbered repository milestones are development history, not public
package releases.

## Current repository state

The current V1 core includes application assembly, HTTP/routing, typed
requests/responses, FormRequest validation, authentication and authorization,
database drivers, Claw ORM, migrations, typed relationships, nested/bounded
named eager loading, Axe views with production validation, cache, storage,
events, jobs, outbound HTTP, notifications, testing, code generation, CLI
tooling, observability primitives, and a realistic foundation reference
application.

The current coordinated publication graph contains **18 publishable crates**.
Rust **1.88** remains the MSRV.

The V1 maturity gate is the source of truth:
[v1-maturity-gate.md](v1-maturity-gate.md).

## Phase 1 — first public v1.0.0

Do not add convenience features merely to increase feature count. The remaining
sequence is:

1. Preserve the documented decision to publish without a pre-release independent audit.
2. Freeze the intended 1.0 public API.
3. Create the final release candidate.
4. Rerun formatting, Clippy, tests, documentation, MSRV, feature matrices,
   live PostgreSQL/MySQL/SQLite checks, fuzzing, load/soak, package checks, and
   clean-consumer verification on the exact release commit.
5. Re-check all 18 crates.io package names and the publication graph.
6. Generate the final 20-subject release evidence set: one source archive,
   18 Cargo package-file lists, and one release manifest.
7. Review public documentation, limitations, changelog, upgrade notes, and
   examples against that same commit.
8. Obtain explicit owner approval.
9. Publish sequentially using the publication runbook.

## Phase 2 — production proving

After 1.0, maturity should come primarily from real applications and operational
feedback rather than rapid expansion of the core.

Priorities:

- operate multiple non-trivial Berserk applications in production;
- collect application-development and operational failure modes;
- maintain SemVer and a conservative deprecation policy;
- publish patch/minor releases from demonstrated needs;
- improve diagnostics and documentation from real support cases;
- track performance and memory regressions over time;
- repeat security review and fuzzing as sensitive boundaries change;
- maintain upgrade paths that are tested by real consumers.

## Phase 3 — ecosystem maturity

Laravel-level ecosystem maturity is not a feature-count target. It requires a
stable framework plus an ecosystem that can succeed without the framework
author manually guiding every user.

Post-1.0 ecosystem work should focus on:

- stable extension contracts for third-party integrations;
- a deliberately small set of maintained official integrations;
- package discovery and compatibility metadata built around Cargo/crates.io,
  not a replacement package manager;
- excellent tutorials, reference applications, starter kits, and deployment
  guides;
- observability and infrastructure integrations;
- stronger application-testing ergonomics and fakes;
- IDE/editor support where Rust Analyzer cannot express Berserk-specific
  concepts;
- production database edge-case coverage;
- public performance history rather than one-off benchmark claims;
- contributor governance, RFC/change review, release cadence, and security
  ownership;
- external maintainers, packages, tutorials, issue reports, and production
  deployments.

The detailed evidence model is in
[ecosystem-maturity.md](ecosystem-maturity.md).

## Future language work

A custom Berserk source language/transpiler is not required for v1 and is not an
ecosystem-maturity shortcut. If pursued later, the Rust-facing public contracts
and shared application/module specification should remain the stable semantic
foundation.

A sensible order is:

1. stable Rust APIs;
2. stable application/module specification;
3. code generation;
4. parser/AST;
5. diagnostics and formatter;
6. language-server/editor integration.

## Principles

Security, correctness, Rust safety, MSRV compatibility, coherent developer
experience, extensibility, explicit application control, and backward
compatibility take priority over copying another framework's syntax.

New abstractions require demonstrated framework or application need. Berserk
should prefer small Rust traits, explicit composition, and replaceable
boundaries over service locators, global mutable registries, reflection, or
hidden framework magic.
