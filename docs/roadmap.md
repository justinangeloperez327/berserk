# Berserk roadmap

Berserk is preparing its first public v1.0.0 release. Repository milestones after the original 1.0 freeze are development history, not public releases.

## First public release — v1.0.0

The first public release consolidates the framework work completed so far: application assembly, HTTP and routing, typed requests and responses, validation, authentication and authorization, database drivers, Claw ORM, migrations and relationships, application services, testing, observability, developer tooling, and production hardening.

The remaining work before publication is driven by repository evidence rather than pre-assigned version buckets. The concrete release decision is defined by [the v1 maturity gate](v1-maturity-gate.md): v1 must not be published until every framework-maturity and release-candidate blocker there is complete.

Current priorities are:

- keep the public surface coherent and beginner-friendly;
- improve framework plumbing where application code still performs framework work manually;
- strengthen extensibility and application control through Rust-native inversion of control;
- preserve explicit execution, type safety, bounded resources, and Rust 1.88 compatibility;
- remove documentation and implementation drift before publication.

## Development history

Earlier development milestones remain useful implementation history. They do not represent public package releases.

Migration and Claw relationship work completed during development is part of the v1.0.0 public-release target.

## How future work is organized

Branches are named for the work they contain, for example `docs-current-contract`, `extensibility-control`, or `request-plumbing`. Branch names do not imply release numbers.

Version numbers are assigned when preparing an actual feature release. Ordinary cleanup, architecture, plumbing, and integration work should not be forced into predetermined version roadmap slots.

## Principles

Security, correctness, Rust safety, MSRV compatibility, coherent developer experience, extensibility, and explicit application control take priority over copying another framework's syntax.

New abstractions require a demonstrated framework need. Berserk should prefer small Rust traits, explicit composition, and replaceable boundaries over service locators, global mutable registries, reflection, or framework magic.
