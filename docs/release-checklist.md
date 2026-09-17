# Release checklist

Publishing is always a manual owner decision. This checklist intentionally contains no publish command.

## Identity and policy

- [x] Confirm crates.io availability for the final `berserk`, `berserk-*`, and `claw-orm` package names. The 2026-09-17 check found no existing index entry for any of the 15 intended package names; see `docs/package-name-availability.md`. Re-check immediately before first publication because names are allocated first-come, first-served and this verification does not reserve them.
- [x] Select and add the MIT license; update publishable package license/repository metadata.
- [x] Configure a private vulnerability-reporting channel and maintainer ownership. Maintainer ownership is formalized in `.github/CODEOWNERS`; `SECURITY.md` and `docs/vulnerability-response.md` define intake, severity, triage, response targets, remediation, and disclosure. The repository owner confirmed on 2026-09-17 that GitHub Private Vulnerability Reporting is enabled.
- [x] Choose supported operating systems and database server versions. `docs/support-policy.md` defines Linux/Ubuntu 24.04 as the Tier-1 host, Windows/macOS development compatibility coverage, PostgreSQL 15-18, MySQL 8.4 LTS, and bundled SQLite. CI enforces the stated platform and database matrix.
- [x] Set the initial package version to `0.1.0` and explicitly mark library/CLI crates publishable while examples and benchmarks remain private.

## Verification

- [x] Pass formatting, compile, Clippy, tests, docs, and independent consumer checks on stable Rust.
- [x] Pass the compile matrix on MSRV Rust 1.88.
- [x] Test no-default-features, every optional feature alone, expected combinations, and all-features.
- [x] Pass PostgreSQL, MySQL, and SQLite contract and migration tests against real databases.
- [x] Run dependency advisory, license, duplicate-version, and source-policy checks.
- [x] Fuzz the current JSON, HTTP-value, route-registration, and multipart input boundaries.
- [x] Run concurrent server load, overload, shutdown-under-load, and prolonged soak tests. `Concurrent load` run 5 on `main` completed a 900-second soak with 16,676,335 attempted/completed requests, zero client errors, zero rejections, and zero server failures.
- [x] Record reproducible sequential latency, throughput, process RSS, and environment data; retain raw evidence.
- [ ] Complete an independent security and API review; resolve or document every finding. An internal 2026-09-17 review fixed outbound HTTP header validation and request-debug target redaction, and refreshed the security/public-API records, but an independent external review is still required before this gate is complete.

## Release artifacts

- [x] Review public API documentation and examples from a clean machine. `docs/clean-machine-review.md` records the review; Package run 26 passed both fresh external-consumer compilation checks on 2026-09-17.
- [x] Review changelog, upgrade notes, known limitations, MSRV, and support policy. `CHANGELOG.md`, `docs/upgrade-notes.md`, `docs/known-limitations.md`, `docs/compatibility.md`, and `docs/support-policy.md` now describe the `0.1.0` candidate consistently, including Rust 1.88 and the current platform/database boundaries.
- [x] Inspect packaged file lists and verify no credentials, local paths, fixtures, or build output are included.
- [ ] Generate checksums and provenance/signing material according to the chosen release platform.
- [ ] Prepare a rollback/yank and security-response plan.

## Owner authorization

- [ ] The owner explicitly approves the exact versions and artifacts.
- [ ] The owner performs publication.
