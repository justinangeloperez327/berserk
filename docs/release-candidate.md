# Berserk v1.0.0 candidate validation

The `Release candidate` workflow runs the existing eight validation workflows
against one exact Git commit: CI, live databases, security, fuzzing, concurrent
load, performance baseline, package/clean-consumer checks, and release artifacts.
Its final `release-gate` job fails if any required workflow fails, is cancelled,
or is skipped. It does not publish, tag, or create a GitHub Release.

The 1.0 public API is frozen. The owner's independent-review deferral remains
in effect; see [release-decision-v1.0.0.md](release-decision-v1.0.0.md). Technical
validation does not constitute an independent audit or permission to publish.

## Run the exact publication candidate

Merge all intended source, documentation, and release-readiness changes first.
Record the final `main` commit, then run **Actions → Release candidate → Run
workflow** from that ref. The workflow file must exist on the default branch
before it can be manually dispatched.

With GitHub CLI, from a clean checkout of the final commit:

```sh
git rev-parse HEAD
gh workflow run release-candidate.yml --ref main -f soak_seconds=300
gh run list --workflow release-candidate.yml --limit 5
```

Confirm the selected run's commit equals the intended publication commit. If
`main` moved before dispatch, run against a release preparation branch pointing
at the intended commit instead. Every checkout pins `github.sha`, so later
branch movement cannot change a job's source. Local reusable workflow references
also resolve from the caller's commit.

Release-workflow pull requests run the same matrix and prolonged soak, but use
GitHub's temporary proposed-merge commit and do not sign attestations. Their
success validates the PR; it is not signed evidence for a later merge commit.

## Candidate requirements and evidence

The preflight requires a clean checkout at the full expected SHA, the synchronized
18-package `1.0.0` publication graph, and a soak duration between **300 and 900
seconds**. It checks the current crates.io visibility of every package name and
its hyphen/underscore-equivalent spellings. Only HTTP 404 counts as unregistered;
network failures, other HTTP errors, malformed responses, or registered names
fail the candidate. Names are not reserved, and publisher permissions are not
verified. Recheck registry names immediately before publication.

The default prolonged soak is five minutes with 16 concurrent clients, in
addition to the existing 20-second smoke soak, overload, and shutdown checks.
Select 900 seconds for a 15-minute run. A successful run records request/error
accounting and peak process RSS; it is not evidence of unlimited uptime or
constant memory over an arbitrary production lifetime.

The workflow retains:

- candidate preflight: exact commit, publication order, registry observations,
  observation time, and selected soak duration;
- the existing load, prolonged-soak, performance, and package-file evidence;
- release evidence with exactly **20 checksum subjects**: the source archive,
  18 package-file lists, and manifest; the candidate preflight is embedded in
  that manifest;
- GitHub/Sigstore provenance for manual runs only; and
- the final per-workflow result record and Actions summary.

Follow [release-provenance.md](release-provenance.md) to verify the 20 checksums
and source attestation. Review the manifest's source commit, publication plan,
registry observations, and all package file lists. Preserve the preflight and
gate result records alongside the signed evidence. A standalone `Release
artifacts` run remains useful for artifact preparation but does not establish
the complete release matrix.

Then complete [release-checklist.md](release-checklist.md) and obtain owner
approval for that exact commit, package set, and reviewed evidence. Publication
uses [publication-runbook.md](publication-runbook.md).

## Workflow isolation

Each reusable workflow has its own concurrency prefix. This prevents called
workflows, which share the caller's workflow context, from cancelling one another.
The existing standalone workflows remain available, with their normal triggers.

Main branch protection is configured separately; see
[branch-protection.md](branch-protection.md).

## GitHub and Cargo references

- [Reusable workflows](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows)
- [Cargo registry index naming rules](https://doc.rust-lang.org/cargo/reference/registry-index.html)
