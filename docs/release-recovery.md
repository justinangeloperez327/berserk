# Release recovery, yank, and security-response runbook

This runbook defines how Berserk responds when publication is interrupted, a published crate is broken, or a security issue is discovered after release. It complements `docs/vulnerability-response.md` and does not automate publication, yanking, or advisory disclosure.

## Principles

- Treat every crates.io version as immutable after publication. Never attempt to replace different source under the same package/version.
- Stop publication as soon as a material inconsistency, packaging defect, security issue, or dependency-order problem is discovered.
- Preserve the exact release commit, release evidence, checksums, provenance, logs, and list of packages already published.
- Prefer a corrected patch release over yanking when users can safely migrate forward.
- Yank only when preventing new dependency resolution is materially safer than leaving the version selectable, or when the publication itself is erroneous.
- Do not treat yanking as deletion. Already downloaded artifacts and existing lockfiles can continue to reference a yanked version.
- If credentials or secrets were exposed, rotate or revoke them immediately. Yanking cannot remove a secret that has already been published.
- Never silently move or recreate a release tag to point at different source for the same version.

## Release owner

The repository owner is the release owner and final authority for publication, yanking, unyanking, and public security disclosure until additional maintainers are explicitly designated.

The release owner should keep the crates.io authentication token outside the repository and CI logs and use the minimum permissions necessary for the release operation.

## Before the first publish command

Confirm all of the following before publishing any crate:

1. The exact release commit is approved by the owner.
2. All release checklist gates that are required before publication are complete.
3. The manual `Release artifacts` run for that exact commit is successful.
4. `SHA256SUMS` verifies successfully and the provenance attestation matches the expected repository, workflow, ref, and commit.
5. `RELEASE-MANIFEST.json` contains the intended 15 packages and a dependency-valid publication order.
6. The package names are re-checked on crates.io immediately before first publication.
7. The release owner records a publication ledger before starting. At minimum record package name, version, publication status, and verification status.

## Partial first-publication failure

If publication stops after only some Berserk crates have reached crates.io:

1. **Stop immediately.** Do not continue publishing additional packages until the failure is understood.
2. Record the exact source commit and which package/version pairs are already present on crates.io.
3. Determine whether the failure is operational only, such as temporary registry propagation, or whether the candidate source/metadata is wrong.
4. If the candidate is still valid and no source or metadata change is needed, publication may resume only from the same approved commit and only after the already-published packages are verified against the intended version and metadata.
5. If any source, dependency, metadata, or security change is required, do not attempt to reuse the already-published version. Prepare a new coordinated patch release.
6. Decide whether already-published versions should remain available or be yanked using the decision rules below.

Because Berserk currently synchronizes the publishable workspace at one version and uses exact internal package versions, a corrective release that changes internal dependencies should normally move the coordinated publishable set to the next patch version rather than creating a mixed-version workspace accidentally.

## Yank decision

Consider yanking a published Berserk version when one or more of these conditions applies:

- the crate cannot build or perform its documented basic contract;
- required files or dependency metadata are materially wrong;
- the publication came from the wrong commit or unintended package contents;
- a Critical or High security issue makes new adoption of that version unsafe;
- an internal exact-version dependency makes the release unusable for new resolution; or
- the release owner determines that continued new selection creates greater operational or security risk than a yank.

Do not yank merely because a newer version exists or because of a minor documentation issue that does not materially affect users.

## Yank procedure

Before yanking, record the reason, affected package/version, incident or advisory reference, and intended replacement version when known.

A crates.io owner can yank an explicit package/version with Cargo, for example:

```sh
cargo yank berserk@0.1.0
```

For a coordinated incident, evaluate every affected package independently and also evaluate the transitive Berserk packages that depend on it. Do not assume yanking one low-level crate is sufficient when higher-level packages still resolve to that exact version.

After yanking:

1. Confirm the version is marked yanked in the registry index.
2. Publish or prepare the corrected patch release in dependency order when a replacement is required.
3. Update the changelog and release notes with the affected version and remediation.
4. For security issues, coordinate disclosure through the GitHub Security Advisory process described in `docs/vulnerability-response.md`.
5. Preserve the original release evidence and incident record. Do not rewrite history to hide the failed release.

## Unyank procedure

Unyank only when the original reason for the yank is proven incorrect or has been resolved without changing the immutable crate contents. The release owner must document why new resolution of the old version is safe again.

Example:

```sh
cargo yank berserk@0.1.0 --undo
```

Do not use unyank as a substitute for publishing a corrected version when the package contents themselves are defective.

## Security incident after publication

For a reported or discovered vulnerability:

1. Use GitHub Private Vulnerability Reporting and keep exploit details private while triage is active.
2. Follow `docs/vulnerability-response.md` for acknowledgement, severity classification, remediation, testing, advisory preparation, and disclosure.
3. Identify every affected Berserk crate and version plus any higher-level package that exposes the vulnerable path.
4. For Critical or High issues, block ordinary release work until the release-security owner records the remediation or explicit residual-risk decision.
5. Add an adversarial regression test whenever practical.
6. Run the relevant normal CI, security, database, fuzz, load, and package checks against the fix.
7. Prepare a new patch release. If the vulnerability crosses exact internal-version boundaries, prefer a coordinated workspace patch release.
8. Yank vulnerable versions only when preventing new selection materially reduces risk; a yank is not a substitute for a patched release or advisory.
9. Publish the GitHub Security Advisory and CVE information when appropriate and coordinated disclosure is ready.

## Secret exposure

If any published package, release artifact, workflow log, commit, issue, or advisory exposes a credential or secret:

1. Revoke or rotate the secret immediately at its issuing system.
2. Determine the exposure window and what authority the secret had.
3. Review audit logs for suspicious use.
4. Remove the secret from active repository surfaces where possible, while assuming copies may already exist.
5. Yank an affected crate only if preventing new resolution is useful, but do not treat the yank as secret removal.
6. Replace the affected release with a clean patch version when package contents contained the secret.

## GitHub release and tag recovery

If a GitHub Release or tag was created for a defective crates.io release:

- preserve the original commit identity and incident evidence;
- do not move the existing version tag to different source;
- annotate the release notes with the known issue or yank status when useful to users;
- create the corrected patch release from a new version/tag after verification; and
- ensure new provenance refers to the corrected source commit rather than reusing previous evidence.

## Recovery verification

A recovery is complete only when the release owner has recorded:

- affected package/version pairs;
- whether each version remains available, was yanked, or was later unyanked;
- root cause and impact;
- replacement version when applicable;
- CI/security verification for the remediation;
- advisory/disclosure status for security incidents;
- release evidence and provenance for the replacement; and
- any follow-up changes needed to prevent recurrence.

The incident record may be public or private depending on security/disclosure requirements, but the maintainer must retain enough information to reconstruct the release decision.