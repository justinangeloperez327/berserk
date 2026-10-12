# Required checks for main

Configure repository **Settings → Branches → Add branch protection rule** for
`main`. This requires repository administration permission.

Enable required status checks, require the branch to be up to date, disallow
force pushes and branch deletion, and apply the rule to administrators. Select
the always-running checks from the standalone `CI` workflow:

- `repository-hygiene`
- `feature-core`
- `feature-services`
- `feature-integrations`
- `quality`
- `msrv`
- `platform-windows-latest`
- `platform-macos-latest`

Select the contexts shown by GitHub from a successful CI run. Do not substitute
the prefixed checks produced by the optional `Release candidate` caller.
Path-filtered workflows should not be required for every PR: unrelated changes
can leave those workflows without a status and block a merge. The complete
release matrix is enforced separately by the candidate's `release-gate` job.

Review requirements depend on the maintainer team. Add required independent PR
reviews only when another maintainer is available to review the author's PRs.
Keep publication approval separate from merge permission: passing required CI
does not authorize uploading crates or creating release tags.
