# Berserk v1.0.0 release decision

Date: 2026-09-23

## Decision

The project owner has elected to publish Berserk v1.0.0 without completing a
pre-release independent API/security review.

This decision is a release-policy risk acceptance. It is **not** evidence that
an independent review occurred, and Berserk must not be represented as
independently audited, penetration-tested, security-certified, or
production-certified.

## Required release conditions

The review deferral does not waive the technical release gates. The exact source
commit published as v1.0.0 must still pass the repository's formatting, Clippy,
tests, documentation, MSRV, feature-matrix, platform, live-database, security,
fuzz, load, package, clean-consumer, and release-artifact checks required by the
release checklist.

The 18 publishable crates must remain version-synchronized at 1.0.0 and be
published in the dependency-safe order produced by
`scripts/release_plan.py --version 1.0.0`.

## Disclosure

Public documentation for v1.0.0 must retain the following facts:

- the framework has internal review and automated security validation;
- no independent pre-release API/security audit was completed;
- no third-party penetration test or security certification is claimed;
- known deployment and security boundaries remain documented in
  `SECURITY.md` and `docs/known-limitations.md`.

## Follow-up

An independent post-release API/security review remains recommended. Findings
from such a review should be handled through compatible patch/minor releases
where possible, with major-version changes reserved for genuinely breaking
public API corrections.
