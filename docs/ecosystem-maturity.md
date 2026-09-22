# Ecosystem maturity

Berserk's V1 goal is a mature framework core. Ecosystem maturity is a separate,
longer-term outcome measured by independent usage, production history,
compatibility, integrations, documentation, and community participation.

Laravel-level maturity cannot be reached by copying Laravel's feature list into
Berserk. A larger framework surface without equivalent production evidence would
increase maintenance and compatibility risk rather than maturity.

## Current position

Berserk currently has:

- a coherent V1 application/runtime architecture;
- an 18-crate coordinated publication graph;
- Claw ORM, Axe views, authentication/authorization, migrations, application
  services, testing, CLI/code generation, and operational primitives;
- a realistic end-to-end foundation application;
- Rust 1.88 MSRV coverage and multi-platform CI;
- security, support, release, recovery, and publication documentation;
- an open independent API/security review gate before the first public release.

This is sufficient to treat the **framework core** as mature enough to prepare
for V1. It is not yet evidence of ecosystem maturity.

## Evidence required for ecosystem maturity

### 1. Production usage

Maturity requires real applications operating for meaningful periods of time.

Evidence should include:

- multiple non-trivial applications using different framework subsystems;
- long-running production workloads rather than only benchmarks;
- incidents and regressions discovered outside synthetic tests;
- upgrade experience across patch and minor releases;
- database, queue, storage, cache, authentication, and deployment behavior
  observed under real traffic.

The framework should change in response to demonstrated application problems,
not hypothetical parity gaps.

### 2. Compatibility history

After v1.0.0:

- maintain SemVer consistently;
- use deprecation before removal whenever practical;
- publish migration guidance for compatibility-affecting changes;
- test representative external consumers;
- maintain the Rust MSRV policy deliberately;
- avoid unnecessary public aliases and duplicate abstractions;
- retain upgrade evidence across several release generations.

Years of predictable compatibility are a maturity signal that cannot be
replaced by one large release.

### 3. Documentation and learning

The documentation should support a developer who has never spoken to a Berserk
maintainer.

Coverage should include installation, project structure, routing, controllers,
requests, validation, authentication, authorization, middleware, Claw models and
relationships, migrations, transactions, pagination, Axe, cache, storage,
events, jobs, notifications, outbound HTTP, testing, deployment, security,
package development, and upgrades.

Beyond reference documentation, the ecosystem should eventually include:

- first-application tutorial;
- complete CRUD tutorial;
- authentication/authorization tutorial;
- API application tutorial;
- production deployment guide;
- Claw relationship/eager-loading guide;
- full sample applications;
- troubleshooting and common-error documentation.

### 4. Extension and package contracts

Third-party integrations must be possible without depending on Berserk
internals.

Stable extension points should cover the boundaries that packages genuinely
need, such as:

- application/service registration;
- configuration;
- routes and middleware;
- migrations;
- events and jobs;
- storage/cache/client adapters;
- authentication/authorization integrations;
- CLI/code-generation hooks only where a stable need exists.

Cargo and crates.io remain the package-distribution mechanism. Berserk should not
create a replacement package manager.

### 5. Official integrations

Maintain a small official set only where central interoperability matters.
Possible categories include remote cache/storage adapters, observability,
authentication protocols, and deployment support.

Official packages should have:

- explicit Berserk compatibility;
- MSRV policy;
- CI against supported framework versions;
- maintained documentation;
- security ownership;
- clear lifecycle/deprecation policy.

Do not move every community integration into the official organization.

### 6. Package discovery

As the ecosystem grows, the Berserk website can provide a package directory on
top of crates.io metadata.

Useful fields include:

- package and maintainer;
- Berserk compatibility range;
- latest release;
- MSRV;
- documentation;
- CI/security status;
- official/community designation.

The directory is discovery metadata, not a registry.

### 7. Developer tooling

The CLI should remain reliable and generated code should compile against the
stable framework contract.

Post-1.0 tooling can expand from demonstrated needs, including:

- richer diagnostics;
- application inspection commands;
- route/config inspection;
- starter templates;
- editor integration for Axe, routes, models, and relationships;
- Berserk-specific diagnostics that Rust Analyzer cannot provide directly.

Tooling should expose ordinary Rust source rather than creating opaque runtime
magic.

### 8. Deployment and operations

Common production paths should be documented and repeatable:

- Linux/systemd;
- containers;
- reverse proxies and TLS termination;
- cloud/container platforms;
- horizontal scaling;
- health/readiness;
- graceful shutdown;
- migrations during deployment;
- queue/scheduled workers;
- logs, metrics, and distributed tracing.

Infrastructure support should prefer standard ecosystem protocols such as
OpenTelemetry and Prometheus-compatible integrations rather than proprietary
framework-only systems.

### 9. Database maturity

Claw and the database layer need real long-term evidence across supported
backends.

Important areas include:

- connection pooling and exhaustion;
- transaction isolation and failure recovery;
- deadlocks/retries;
- large migrations;
- database-specific types;
- indexes and composite keys;
- large eager-loading workloads;
- replication/read-routing where supported later;
- driver upgrades and server-version changes.

Database behavior should remain explicit when PostgreSQL, MySQL, and SQLite
cannot share one semantic guarantee.

### 10. Testing maturity

Application testing should become one of Berserk's strongest developer
experiences.

Useful future evidence includes stable helpers/fakes for:

- HTTP requests/responses;
- authentication;
- database assertions/factories;
- mail/notifications;
- events/jobs;
- storage/cache;
- outbound HTTP;
- time-dependent behavior.

Test helpers must exercise real framework boundaries instead of maintaining a
second incompatible application model.

### 11. Performance history

Performance maturity requires trends, not isolated headline benchmarks.

Track comparable evidence for:

- routing;
- JSON;
- Claw queries/eager loading;
- Axe rendering;
- middleware;
- concurrent connections;
- queue throughput;
- process memory;
- p50/p95/p99 latency;
- cold start where relevant.

Regression detection is more valuable than optimizing purely for benchmark
rankings.

### 12. Security history

The independent V1 review is the beginning of security maturity, not the end.

Long-term maturity requires:

- repeat independent reviews when sensitive boundaries change;
- dependency/security advisory handling;
- parser and protocol fuzzing;
- regression tests for disclosed vulnerabilities;
- documented CVE/advisory handling if needed;
- reliable secret/log redaction;
- SQL injection, XSS, SSRF, path traversal, session/CSRF, and HTTP-framing
  review as relevant;
- timely patch releases and transparent disclosure.

### 13. Governance and maintainership

A mature ecosystem cannot depend on one person making every technical,
security, package, and release decision forever.

As contributors grow, establish:

- change/RFC criteria for major public APIs;
- review ownership by subsystem;
- release cadence;
- security ownership;
- official-package ownership;
- contribution standards;
- process for deprecated or abandoned integrations.

Berserk should remain opinionated; community size is not a reason to accept
every proposed abstraction.

### 14. Independent community

The strongest ecosystem evidence is activity not initiated by the original
author:

- external production users;
- third-party packages;
- independent tutorials and examples;
- bug reports from unfamiliar workloads;
- contributors reviewing and repairing code;
- questions answered by other users;
- companies maintaining deployments;
- maintainers who can release supported components independently.

When developers who have never interacted with the framework author can build,
operate, debug, upgrade, and extend Berserk successfully, the ecosystem is
becoming mature.

## Practical maturity stages

### Stage A — V1 core

Target: first public v1.0.0.

Evidence: current V1 maturity and release gates.

### Stage B — production-proven

Target: several real applications, repeated patch/minor releases, documented
incidents, stable upgrade paths, and sustained performance/security evidence.

### Stage C — package ecosystem

Target: stable extension contracts, useful official integrations, credible
third-party packages, starter applications, deployment integrations, and package
discovery.

### Stage D — community maturity

Target: independent maintainers, organizations using Berserk in production,
community-created learning material and packages, and a multi-year compatibility
and security history.

There is no defensible shortcut from Stage A to Stage D.

## Development allocation after V1

As a default after the first stable release, Berserk should bias effort toward
reliability and adoption rather than framework expansion:

- roughly 80%: reliability, documentation, tooling, integrations, real
  applications, operations, security, and ecosystem;
- roughly 20%: new framework capabilities justified by demonstrated gaps.

This ratio is guidance, not a release rule. The governing principle is that
post-V1 features should follow evidence from real users and applications.
