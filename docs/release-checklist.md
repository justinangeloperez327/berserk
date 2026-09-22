# Release checklist

Berserk v1.0.0 is the intended first public stable release. It has not yet been
published.

This checklist distinguishes **repository baseline evidence** from
**exact-release-candidate evidence**. A historical green workflow proves that a
capability exists; it does not authorize publishing a later commit.

Publishing is always a manual owner decision. This checklist intentionally
contains no automatic publish step.

## Current state

- The V1 framework core is implemented.
- Normal repository CI is green on current `main`.
- The independent API/security review remains open.
- The formal 1.0 public API freeze has not yet been completed.
- The current publication graph contains **18 publishable crates**.
- The release-artifact workflow expects **20 checksum subjects**: one source
  archive, 18 package-file lists, and one release manifest.
- Historical package-name evidence from 2026-09-17 covered only 15 names and is
  not sufficient for the current 18-crate graph.

The source of truth for release blocking is
[v1-maturity-gate.md](v1-maturity-gate.md).

## Repository baseline already established

These items describe capabilities/policies that exist in the repository. They
still need to be exercised again where the exact-candidate sections below say
so.

- [x] MIT license and publishable package license/repository metadata are
      defined.
- [x] GitHub Private Vulnerability Reporting and maintainer ownership are
      documented.
- [x] The support policy defines the intended operating-system, Rust, and
      database matrix.
- [x] Rust 1.88 MSRV validation exists.
- [x] Stable-Rust formatting, compilation, Clippy, tests, documentation,
      feature-matrix, and clean-consumer validation exist in CI.
- [x] Live PostgreSQL/MySQL/SQLite validation exists.
- [x] Dependency advisory/license/source-policy checks exist.
- [x] Fuzz, concurrent-load, overload, shutdown, soak, package, and
      release-artifact workflows exist.
- [x] The foundation reference application exercises migrations, generated-style
      CRUD, validation, relationships, authentication, authorization, Axe, and
      application tests together.
- [x] The release planner validates a synchronized **18-package** publication
      graph and derives dependency-safe publication order.
- [x] Release recovery, yank/security response, publication, and provenance
      procedures are documented.

## Framework maturity gate

- [ ] Complete the independent API/security review against a specific candidate
      commit and record the reviewer, findings, dispositions, verification, and
      conclusion in `docs/independent-api-security-review.md`.
- [ ] Resolve or explicitly document every release-blocking finding from that
      review.

Internal review, CI, fuzzing, and owner approval do not self-satisfy the
independence requirement.

## Exact release-candidate identity

Complete these only after the independent review is closed and the intended
1.0 API is frozen.

- [ ] Record the exact release-candidate commit.
- [ ] Confirm all 18 publishable crates are version-synchronized at `1.0.0`.
- [ ] Confirm all internal publishable dependencies use the intended exact
      version.
- [ ] Confirm examples and benchmarks remain non-publishable.
- [ ] Re-check crates.io name and normalization-equivalent availability for all
      18 intended package names immediately before publication.
- [ ] Verify `scripts/release_plan.py --version 1.0.0` reports exactly 18
      publishable packages and a dependency-safe order.

Do not use the historical 2026-09-17 package-name check as final evidence; it
predates `berserk-codegen`, `berserk-macros`, and `berserk-axe` in the
current publishable graph.

## Exact release-candidate verification

Run all checks against the exact commit intended for publication.

- [ ] `cargo fmt --all -- --check`.
- [ ] Stable Rust compile/check across workspace/all targets/all features.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Workspace tests with the intended locked dependency graph.
- [ ] Documentation build.
- [ ] Rust 1.88 MSRV check.
- [ ] No-default-features and every supported independent/combined feature
      matrix.
- [ ] Windows and macOS platform checks.
- [ ] Live PostgreSQL, MySQL, and SQLite contract/migration checks.
- [ ] Dependency advisory, license, duplicate-version, and source-policy checks.
- [ ] Current fuzz targets.
- [ ] Concurrent load, overload, shutdown-under-load, and soak checks.
- [ ] Generator verification.
- [ ] Package-file inspection.
- [ ] Clean external consumer compile using the documented public surface.
- [ ] Verify `Cargo.lock` remains unchanged by release validation.

Historical successful runs may be cited as engineering history, but they do not
replace this exact-commit rerun.

## Exact release documentation review

- [ ] Review `README.md`, public API documentation, architecture/design
      documentation, support policy, security policy, known limitations, and
      examples against the exact candidate.
- [ ] Review `CHANGELOG.md`, `docs/v1.0.0.md`, and upgrade notes for
      publication wording.
- [ ] Confirm no document claims that Berserk is already independently audited,
      production-certified, or ecosystem-mature.
- [ ] Confirm the current 18-crate package graph is used consistently in release
      documentation.
- [ ] Confirm the final installation examples use the intended crates.io
      version/features.

## Exact release artifacts

- [ ] Run `Release artifacts` manually against the exact final commit.
- [ ] Verify the evidence contains exactly **20** checksum subjects:
      one source archive, 18 package-file lists, and one release manifest.
- [ ] Run `sha256sum -c SHA256SUMS` successfully.
- [ ] Verify GitHub/Sigstore provenance for the exact source/repository/workflow.
- [ ] Review all 18 package file lists for unexpected source, credentials,
      generated/local-only files, or build output.
- [ ] Review `RELEASE-MANIFEST.json` for exact commit, toolchain, package
      versions, dependencies, publish layers, and publication order.
- [ ] Retain the evidence bundle with the release record.

Any older provenance/checksum run is historical evidence only. If the final
source commit differs, regenerate evidence.

## Owner authorization

- [ ] The owner explicitly approves the exact source commit, 1.0.0 package set,
      independent-review disposition, and final release evidence.
- [ ] Publication is performed sequentially according to
      `docs/publication-runbook.md`.
- [ ] Every published package/version is verified remotely from outside the
      workspace.
- [ ] A fresh crates.io-only consumer is compiled.
- [ ] The immutable `v1.0.0` tag and GitHub Release are created only after the
      coordinated crates.io publication is verified.
- [ ] The completed publication ledger and release evidence are retained.
