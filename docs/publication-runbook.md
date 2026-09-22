# Berserk publication runbook

This runbook is for the repository owner or an explicitly authorized release maintainer publishing a coordinated Berserk release to crates.io.

It does **not** replace the release checklist, independent security/API review, or owner approval. Do not publish until every required pre-publication gate is complete.

## Publication model

Berserk publishes 18 coordinated crates. Internal workspace dependencies use exact versions, so first publication must proceed in dependency order. A downstream crate cannot be packaged or published until the internal crate version it depends on is visible in the crates.io index.

`scripts/release_plan.py` is the source of truth for the dependency-safe order. The same planner is used by `.github/workflows/release-artifacts.yml` when creating release evidence.

The planner is deliberately read-only. It never publishes, yanks, tags, or changes a registry.

## Required preconditions

Before the first `cargo publish` command:

1. The independent API/security review is complete and every finding has a recorded disposition.
2. The release checklist has no unresolved pre-publication gate.
3. The owner has approved the exact version and exact source commit.
4. The worktree is clean and checked out at that approved commit.
5. The manual `Release artifacts` workflow has been rerun for that exact final commit after all review fixes/documentation changes are merged.
6. The release evidence for that commit has verified checksums and GitHub/Sigstore provenance.
7. Package-name availability has been rechecked immediately before publication.
8. The crates.io account is ready and the release credential is available only in the maintainer's secure environment.
9. A release record has been prepared with the exact commit, version, package order, and publication results.

Do not use `--allow-dirty` to bypass source-state checks for a release.

## Validate the release plan

From the repository root:

```sh
python3 scripts/release_plan.py --version VERSION
```

The command must report exactly 18 publishable packages and a dependency-safe order.

To render the exact publication commands without executing them:

```sh
python3 scripts/release_plan.py --version VERSION --format commands
```

To retain the machine-readable plan with the release evidence:

```sh
python3 scripts/release_plan.py --version VERSION --format json \
  --output target/release-plan-VERSION.json
```

Review the plan before continuing. If the package count, version, dependency graph, or order is unexpected, stop and fix the repository rather than editing the generated order manually.

## Authenticate

Authenticate to crates.io in a secure local shell using Cargo's supported authentication mechanism. Do not paste the token into source files, issue comments, workflow logs, shell history intended for sharing, or the publication record.

If a release credential is exposed, revoke or rotate it immediately and follow `docs/release-recovery.md`.

## Publish sequentially

Run **one** generated `cargo publish --locked -p <package>` command at a time, in the exact planner order.

After each successful command:

1. Record the package, version, time, and result in the publication record.
2. Confirm Cargo completed its registry-index wait before starting the next dependent crate.
3. Continue only from the same approved source commit.

Cargo's publish command uploads the package and then polls for the package to appear in the registry index. A timeout while polling does not prove the upload failed.

### If a publish command times out or returns an ambiguous registry error

**Stop. Do not immediately retry the same version.**

Verify the exact package/version from outside the Berserk workspace so Cargo cannot satisfy the query from the local package:

```sh
cd /tmp
cargo info PACKAGE@VERSION --registry crates-io
```

You may also verify the version directly on crates.io.

- If the exact version is present, record it as published and continue only after its index entry is usable.
- If it is not present, investigate the original failure before retrying.
- If source or metadata must change, stop the coordinated release. Published versions are immutable; follow `docs/release-recovery.md` and prepare the next coordinated patch version rather than attempting to replace the uploaded version.

## Never skip dependency order

Do not publish a higher-level crate merely because its source builds locally. During first publication its internal exact-version dependencies must already exist in the registry.

If the planner detects a dependency cycle, unexpected publishable crate, disabled expected crate, or version mismatch, it exits non-zero. Treat that as a release blocker.

## Verify the completed crates.io set

After all 18 publishes are recorded as successful, verify every exact version from outside the workspace. At minimum verify the facade and representative lower-level crates:

```sh
cd /tmp
cargo info berserk@VERSION --registry crates-io
cargo info berserk-core@VERSION --registry crates-io
cargo info claw-orm@VERSION --registry crates-io
```

The publication record should contain remote-verification status for all 18 packages, not only these examples.

## Fresh crates.io consumer test

Create a new project outside the repository that depends only on crates.io packages, not local paths or Git revisions.

Example dependency:

```toml
[dependencies]
berserk = { version = "VERSION", features = ["sqlite", "claw", "auth", "openapi"] }
```

Compile a minimal application using the documented quick-start API with `cargo check --locked` (or generate and then retain its lockfile before the locked check).

This test must prove that the public facade resolves entirely from the published registry set.

## Tag and GitHub Release

Only after the coordinated crates.io set and fresh external consumer are verified:

1. Confirm the release commit is still the exact approved/provenance commit.
2. Create the immutable version tag for that commit (for version `X.Y.Z`, use `vX.Y.Z`).
3. Push the tag.
4. Create the GitHub Release from that tag using the reviewed changelog/release notes.
5. Attach or link the retained release evidence as appropriate.
6. Never move an existing version tag to different source. If a published release is defective, follow the patch-release recovery process instead.

## Close the release record

A first release is complete only after the publication record contains:

- exact release commit and tag;
- exact version;
- release-evidence workflow run and attestation reference;
- generated publication plan;
- all 18 package/version publication results;
- all 18 remote verification results;
- fresh crates.io consumer result;
- GitHub Release reference;
- any exceptions/incidents and their disposition; and
- owner completion signoff.

Retain the record with the release evidence so a future maintainer can reconstruct exactly what was published and from which commit.

## Cargo references

The process above follows Cargo's documented behavior for `cargo publish`, package ID specifications, and registry publication. In particular, Cargo documents that publish waits for the uploaded package to appear in the index and that a timeout in that wait does not affect the upload itself.
