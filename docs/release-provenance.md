# Release checksums and provenance

Berserk prepares release-candidate source package archives through `.github/workflows/release-artifacts.yml`. The workflow does **not** publish crates, create a GitHub Release, or upload anything to crates.io.

## Release artifact set

A successful run produces the 15 intended publishable `.crate` archives plus:

- `RELEASE-MANIFEST.json` — repository, source commit/ref, workflow run, Rust/Cargo versions, and package/version inventory.
- `SHA256SUMS` — SHA-256 digests for every `.crate` archive and the release manifest.
- `provenance.sigstore.json` — generated only for a manual `workflow_dispatch` run; this is the Sigstore bundle returned by GitHub artifact attestation.

Pull-request runs validate package generation and checksums only. They intentionally do not create signed attestations. Manual release-artifact runs use `actions/attest@v4` to create SLSA build-provenance attestations for every subject listed in `SHA256SUMS`.

## Why signing is manual

Release provenance should describe artifacts that are deliberately being evaluated for release, not every routine CI build. The workflow therefore creates attestations only when a maintainer explicitly runs `Release artifacts` through `workflow_dispatch`.

The workflow still performs the same package and checksum construction on relevant pull requests so changes to the release process are testable before merge.

## Integrity verification

After downloading the workflow artifact, verify the checksums from inside the extracted artifact directory:

```sh
sha256sum -c SHA256SUMS
```

All 16 entries must report `OK`: 15 `.crate` archives plus `RELEASE-MANIFEST.json`.

## Provenance verification

For an online verification against the GitHub repository, verify any release archive with GitHub CLI:

```sh
gh attestation verify berserk-0.1.0.crate \
  --repo justinangeloperez327/berserk \
  --signer-workflow justinangeloperez327/berserk/.github/workflows/release-artifacts.yml
```

The verification should establish that the artifact digest is covered by a valid SLSA provenance attestation issued for this repository and signer workflow.

For offline verification, keep the generated `provenance.sigstore.json` bundle and obtain the current GitHub/Sigstore trusted root while online:

```sh
gh attestation trusted-root > trusted_root.jsonl
```

Then use the local bundle and trusted root with `gh attestation verify` in the offline environment. Trusted roots should be refreshed whenever newly signed release material is moved into an offline environment.

## Release procedure

For an intended release commit or tag:

1. Ensure all normal release gates are green and the exact source commit has owner approval for artifact preparation.
2. Run the `Release artifacts` workflow manually from that exact ref.
3. Download `berserk-release-artifacts-<sha>` from the successful workflow run.
4. Run `sha256sum -c SHA256SUMS`.
5. Verify at least the main `berserk-<version>.crate` attestation with `gh attestation verify`, including the expected repository and signer workflow.
6. Review `RELEASE-MANIFEST.json` and confirm the commit, package versions, Rust toolchain, and package inventory are the intended release candidate.
7. Retain the checksums and attestation bundle with the release evidence.

This provenance proves where and how the artifacts were produced; it is not a claim that the artifacts are vulnerability-free or independently security-audited.
