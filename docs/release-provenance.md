# Release checksums and provenance

Berserk prepares pre-publication release evidence through `.github/workflows/release-artifacts.yml`. The workflow does **not** publish crates, create a GitHub Release, or upload anything to crates.io.

## Why the evidence is source-based before the first publication

Berserk is a multi-crate workspace whose publishable crates depend on one another. Stable Cargo rewrites versioned path dependencies to registry dependencies when creating a distributable package. During an initial coordinated publication, downstream crates cannot be packaged against registry dependencies until required internal versions are visible in the registry.

The release workflow therefore attests the exact source snapshot and package inputs before publication. Actual `.crate` archives are created by Cargo during the later sequential publication process, after each required internal dependency is available in the registry.

Berserk deliberately does not depend on nightly-only workspace packaging for the release process.

## Release evidence set

A successful run produces:

- `berserk-source-<commit>.tar.gz` — deterministic source snapshot from the exact Git commit selected for the run.
- `package-files/<package>-<version>.txt` — Cargo's package file list for each of the 15 intended publishable crates.
- `RELEASE-MANIFEST.json` — repository, source commit/ref, workflow run, Rust/Cargo versions, package/version inventory, internal dependency graph, and calculated publication order.
- `SHA256SUMS` — SHA-256 digests for the source snapshot, all 15 package-file lists, and the release manifest.
- `provenance.sigstore.json` — generated only for a manual `workflow_dispatch` run; this is the Sigstore bundle returned by GitHub artifact attestation.

Pull-request runs validate evidence generation and checksums only. They intentionally do not create signed attestations. Manual release-artifact runs use `actions/attest@v4` to create GitHub build-provenance attestations for every subject listed in `SHA256SUMS`.

## Package publication order

`RELEASE-MANIFEST.json` derives a topological publication order from Cargo metadata. A crate is listed only after its publishable internal dependencies.

For the first crates.io release, publish one crate at a time in that order and wait until each package/version is visible in the crates.io index before packaging/publishing a dependent crate. Do not bypass dependency resolution by editing packaged manifests or by substituting Git/path-only dependencies.

The manifest's publication order is release evidence, not an automatic publisher. Publication remains an explicit owner action.

## Integrity verification

After downloading the workflow artifact, verify the checksums from inside the extracted artifact directory:

```sh
sha256sum -c SHA256SUMS
```

A complete evidence bundle contains 17 checksum subjects:

- one source archive;
- 15 Cargo package-file lists; and
- one release manifest.

All entries must report `OK`.

## Provenance verification

For online verification against the GitHub repository, verify the source snapshot with GitHub CLI:

```sh
gh attestation verify berserk-source-<commit>.tar.gz \
  --repo justinangeloperez327/berserk \
  --signer-workflow justinangeloperez327/berserk/.github/workflows/release-artifacts.yml
```

The verification should establish that the artifact digest is covered by a valid provenance attestation issued for this repository and signer workflow.

For offline verification, retain `provenance.sigstore.json` and obtain the current GitHub/Sigstore trusted root while online:

```sh
gh attestation trusted-root > trusted_root.jsonl
```

Then use the local bundle and trusted root with `gh attestation verify` in the offline environment. Trusted roots should be refreshed whenever newly signed release material is moved into an offline environment.

## First-release procedure

For the intended release commit or tag:

1. Ensure all normal release gates are green and the exact source commit has owner approval for artifact preparation.
2. Run the `Release artifacts` workflow manually from that exact ref.
3. Download `berserk-release-evidence-<sha>` from the successful workflow run.
4. Run `sha256sum -c SHA256SUMS`.
5. Verify the source archive attestation with `gh attestation verify`, including the expected repository and signer workflow.
6. Review `RELEASE-MANIFEST.json` and confirm the commit, package versions, Rust toolchain, internal dependencies, and publication order.
7. Review the 15 package file lists for unexpected source, credential, generated, or local-only files.
8. Publish packages sequentially in the manifest order, waiting for each internal dependency to appear in the registry before proceeding to dependents.
9. Retain the checksum file, manifest, package-file lists, source snapshot, and attestation bundle with the release evidence.

After publication, crates.io supplies registry checksums for the uploaded package archives. Those registry checksums should be captured with the final release record so the published `.crate` contents can be tied back to the approved source release evidence.

This provenance proves which source and package inputs were approved and attested before publication; it is not a claim that Berserk is vulnerability-free or independently security-audited.
