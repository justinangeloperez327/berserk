# Release checklist

Publishing is always a manual owner decision. This checklist intentionally contains no publish command.

## Identity and policy

- [ ] Select final project and package names; confirm registry and repository availability.
- [ ] Select and add a license; update every package's license/repository metadata.
- [ ] Configure a private vulnerability-reporting channel and maintainer ownership.
- [ ] Choose supported operating systems and database server versions.
- [ ] Replace version `0.0.0` and deliberately set package publication flags.

## Verification

- [x] Pass formatting, compile, Clippy, tests, docs, and independent consumer checks on stable Rust.
- [x] Pass the compile matrix on MSRV Rust 1.88.
- [x] Test no-default-features, every optional feature alone, expected combinations, and all-features.
- [ ] Pass PostgreSQL, MySQL, and SQLite contract and migration tests against real databases.
- [x] Run dependency advisory, license, duplicate-version, and source-policy checks.
- [ ] Fuzz all untrusted parsers and run server load, overload, shutdown, and soak tests.
- [ ] Record reproducible latency, throughput, memory, and environment data.
- [ ] Complete an independent security and API review; resolve or document every finding.

## Release artifacts

- [ ] Review public API documentation and examples from a clean machine.
- [ ] Review changelog, upgrade notes, known limitations, MSRV, and support policy.
- [ ] Inspect packaged file lists and verify no credentials, local paths, fixtures, or build output are included.
- [ ] Generate checksums and provenance/signing material according to the chosen release platform.
- [ ] Prepare a rollback/yank and security-response plan.

## Owner authorization

- [ ] The owner explicitly approves the exact versions and artifacts.
- [ ] The owner performs publication.
