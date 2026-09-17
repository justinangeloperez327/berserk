# Berserk publication record — VERSION

Copy this template for each crates.io release. Keep the record aligned with the exact approved release commit and do not include credentials or secrets.

## Release identity

- Version: `VERSION`
- Release commit: `COMMIT_SHA`
- Git tag: `TAG`
- Release owner: `OWNER`
- Owner approval recorded: `yes/no`
- Independent review record: `REFERENCE`
- Release-evidence workflow run: `RUN_REFERENCE`
- Provenance/attestation reference: `ATTESTATION_REFERENCE`
- Publication-plan file/hash: `REFERENCE`
- Publication started: `TIMESTAMP`
- Publication completed: `TIMESTAMP`

## Package ledger

Record each package immediately after the registry operation and again after remote verification.

| Order | Package | Version | Publish result/time | Remote verification | Notes |
| ---: | --- | --- | --- | --- | --- |
| 1 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 2 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 3 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 4 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 5 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 6 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 7 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 8 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 9 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 10 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 11 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 12 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 13 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 14 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |
| 15 | `TBD_FROM_RELEASE_PLAN` | `VERSION` |  |  |  |

Populate the package column from:

```sh
python3 scripts/release_plan.py --version VERSION --format tsv
```

Do not reorder the ledger manually.

## Fresh registry-consumer verification

- Test directory/environment: `REFERENCE`
- Dependency resolved from crates.io only: `yes/no`
- Feature set tested: `FEATURES`
- `cargo check` result: `PASS/FAIL`
- Lockfile retained/reference: `REFERENCE`
- Notes: `NOTES`

## Tag and GitHub Release

- Tag created after registry verification: `yes/no`
- Tag points to exact release commit: `yes/no`
- GitHub Release reference: `REFERENCE`
- Release notes/changelog verified: `yes/no`

## Incidents or exceptions

Record any timeout, partial publication, yank, unyank, registry propagation problem, metadata discrepancy, credential event, or other deviation.

| ID | Event | Affected package/version | Action | Resolution/reference |
| --- | --- | --- | --- | --- |
|  |  |  |  |  |

If an incident changes source or metadata after any crate has been published, stop the release and follow `docs/release-recovery.md`.

## Final owner signoff

- All 15 package/version pairs remotely verified: `yes/no`
- Fresh crates.io consumer passed: `yes/no`
- Release tag/GitHub Release completed: `yes/no`
- Incidents closed or explicitly accepted: `yes/no`
- Owner completion decision: `COMPLETE/INCOMPLETE`
- Owner/date: `NAME / DATE`
