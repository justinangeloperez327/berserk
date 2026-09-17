# Independent security and public API review record

Copy this template to `docs/external-review-record.md` only after the independent review is complete. Do not use the existence of this template as evidence that the release gate is satisfied.

## Review identity

- Reviewer / organization:
- Review start date:
- Review completion date:
- Initial reviewed commit:
- Final reviewed commit:
- Candidate version: `0.1.0`
- Material scope exclusions:

## Methodology

Summarize source review, tests, tools, adversarial cases and any environment-specific verification performed.

## Coverage

Reference the completed coverage table from `docs/external-review-findings-template.md` or reproduce it here. Every mandatory area from `docs/external-review-package.md` must have a recorded result or explicit exclusion.

## Findings summary

| ID | Severity | Status | Affected area | Resolution / residual risk |
| --- | --- | --- | --- | --- |
| | | | | |

## Release-blocking disposition

Document the final disposition of every Critical, High and Medium finding and every Documentation/API finding that materially affects the published contract.

For accepted risks, record the release owner's rationale and the exact documentation that communicates the remaining limitation.

## Fix verification

List each finding that required source changes and record:

- fix commit / PR;
- regression test or other verification;
- relevant CI/security/database/fuzz/load/package evidence;
- reviewer re-review result.

## Residual risks and limitations

Record risks that remain after remediation and whether they are already represented in `SECURITY.md`, `docs/known-limitations.md`, `docs/public-api.md`, or other release documentation.

## Reviewer conclusion

Include the final reviewer conclusion from `docs/external-review-signoff-template.md`, the final reviewed commit, and any qualifications.

## Release-owner acknowledgement

Record that the release owner reviewed all findings and dispositions. This acknowledgement does not replace the separate owner publication approval in `docs/release-checklist.md`.

- Release owner:
- Date:
- Decision:

## Evidence references

Link or reference the retained findings report, sign-off, relevant pull requests, private advisory records when applicable, and CI/review evidence required to reconstruct the decision.
