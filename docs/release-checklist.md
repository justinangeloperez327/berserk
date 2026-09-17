# Release checklist

Publishing is always a manual owner decision. This checklist intentionally contains no publish command.

## Identity and policy

- [ ] Confirm crates.io availability for the final `berserk`, `berserk-*`, and `claw-orm` package names.
- [x] Select and add the MIT license; update publishable package license/repository metadata.
- [ ] Configure a private vulnerability-reporting channel and maintainer ownership.
- [ ] Choose supported operating systems and database server versions.
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
- [x] Complete an independent security and API review; resolve or document every finding. The 2026-09-17 release review fixed outbound HTTP client header control-byte validation and records residual deployment/release boundaries in `docs/security-review.md`; this is not a third-party audit or penetration test.

## Release artifacts

- [ ] Review public API documentation and examples from a clean machine.
- [ ] Review changelog, upgrade notes, known limitations, MSRV, and support policy.
- [x] Inspect packaged file lists and verify no credentials, local paths, fixtures, or build output are included.
- [ ] Generate checksums and provenance/signing material according to the chosen release platform.
- [ ] Prepare a rollback/yank and security-response plan.

## Owner authorization

- [ ] The owner explicitly approves the exact versions and artifacts.
- [ ] The owner performs publication.
