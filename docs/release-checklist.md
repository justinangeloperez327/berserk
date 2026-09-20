# Release checklist

Use this checklist for each Berserk release candidate. Historical evidence from an earlier release does not automatically satisfy a later release gate; rerun release-specific validation against the exact candidate commit.

Publishing is always a manual owner decision. This checklist intentionally contains no publish command.

## Identity and policy

- [x] Confirm crates.io availability for the final `berserk`, `berserk-*`, and `claw-orm` package names. The 2026-09-17 check found no existing index entry for any of the 15 intended package names; see `docs/package-name-availability.md`. Re-check immediately before first publication because names are allocated first-come, first-served and this verification does not reserve them.
- [x] Select and add the MIT license; update publishable package license/repository metadata.
- [x] Configure a private vulnerability-reporting channel and maintainer ownership. Maintainer ownership is formalized in `.github/CODEOWNERS`; `SECURITY.md` and `docs/vulnerability-response.md` define intake, severity, triage, response targets, remediation, and disclosure. The repository owner confirmed on 2026-09-17 that GitHub Private Vulnerability Reporting is enabled.
- [x] Choose supported operating systems and database server versions. `docs/support-policy.md` defines Linux/Ubuntu 24.04 as the Tier-1 host, Windows/macOS development compatibility coverage, PostgreSQL 15-18, MySQL 8.4 LTS, and bundled SQLite. CI enforces the stated platform and database matrix.
- [x] Confirm all publishable crates use the intended coordinated release version and that examples/benchmarks remain non-publishable where required. The v1.0.0 release branch aligns workspace packages and internal dependency requirements to 1.0.0; the release planner/package workflow remains the executable verification gate.

## Verification

- [x] Pass formatting, compile, Clippy, tests, docs, and independent consumer checks on stable Rust.
- [x] Pass the compile matrix on MSRV Rust 1.88.
- [x] Test no-default-features, every optional feature alone, expected combinations, and all-features.
- [x] Pass PostgreSQL, MySQL, and SQLite contract and migration tests against real databases.
- [x] Run dependency advisory, license, duplicate-version, and source-policy checks.
- [x] Fuzz the current JSON, HTTP-value, route-registration, and multipart input boundaries.
- [x] Run concurrent server load, overload, shutdown-under-load, and prolonged soak tests. `Concurrent load` run 5 on `main` completed a 900-second soak with 16,676,335 attempted/completed requests, zero client errors, zero rejections, and zero server failures.
- [x] Record reproducible sequential latency, throughput, process RSS, and environment data; retain raw evidence.
- [ ] Complete any independent security/API review required by the release policy and resolve or document every finding. Internal review does not count as independent review.

## Release artifacts

- [x] Review public API documentation and examples from a clean machine. `docs/clean-machine-review.md` records the review; Package run 26 passed both fresh external-consumer compilation checks on 2026-09-17.
- [x] Review changelog, upgrade notes, known limitations, MSRV, support policy, public API, and examples for the exact release candidate. The v1.0.0 documentation pass aligns the stable routing/authentication surface, operational fixes, Rust 1.88 MSRV, and support boundaries with the release branch.
- [x] Inspect packaged file lists and verify no credentials, local paths, fixtures, or build output are included.
- [x] Generate checksums and provenance/signing material according to the chosen release platform. Manual `Release artifacts` run 4 on `main` commit `0e882ce2c852c07a6b23a3dfcd68e05aad28cec8` successfully verified all 17 checksum subjects, generated GitHub/Sigstore build provenance for all 17 subjects, uploaded the attestation to the repository and Rekor transparency log, retained the provenance bundle, and uploaded the 90-day release-evidence artifact. See `docs/release-provenance.md`. A new manual run is required for the exact final release commit if that commit differs from the recorded evidence commit.
- [x] Prepare a rollback/yank and security-response plan. `docs/release-recovery.md` defines partial-publication recovery, yank/unyank decision rules, immutable-version handling, coordinated patch-release recovery, secret-exposure response, tag/release handling, and recovery verification; `docs/vulnerability-response.md` defines the linked private security-response process.
- [x] Prepare publication operator tooling. `scripts/release_plan.py` validates the synchronized publishable set and derives dependency-safe publication order; `docs/publication-runbook.md` defines the sequential crates.io procedure and stop conditions. Maintain a release ledger containing the exact commit, version, package order, timestamps, and publication results. The planner is read-only and does not publish anything.

## Owner authorization

- [ ] The owner explicitly approves the exact versions, final source commit, and final release evidence after the independent review gate is complete.
- [ ] The owner performs publication according to `docs/publication-runbook.md` and retains a completed publication record.
