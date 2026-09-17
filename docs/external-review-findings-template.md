# Independent review findings template

Use one section per finding. Keep enough detail that the release owner can reproduce the issue, understand affected trust boundaries, and verify the disposition without relying on private reviewer notes.

## Review metadata

- Reviewer / organization:
- Review start date:
- Review end date:
- Initial reviewed commit:
- Final reviewed commit:
- Methodology / tools:
- Scope exclusions:

## Coverage summary

For every mandatory area in `docs/external-review-package.md`, record either finding IDs or `No material finding observed`.

| Area | Finding IDs / result | Notes |
| --- | --- | --- |
| HTTP parsing, routing and request extraction | | |
| Sanitization and validation order | | |
| Database and Claw ORM | | |
| Authentication, sessions and authorization | | |
| Storage | | |
| Outbound HTTP, notifications and SSRF boundary | | |
| Server concurrency, overload and shutdown | | |
| Cache, jobs, events and side effects | | |
| OpenAPI, CLI and generated material | | |
| Diagnostics and secret redaction | | |
| Dependency and unsafe-code boundary | | |
| Public API contract and ergonomics | | |

---

## Finding EXT-XX — <short title>

- Severity: Critical / High / Medium / Low / Documentation/API / Informational
- Status: Open / Fixed / Accepted risk / Not reproducible / Duplicate
- Affected crate(s):
- Affected version / commit:
- Security boundary or API contract:
- Attacker prerequisites / misuse prerequisites:

### Summary

Describe the problem and why it matters.

### Technical details

Identify the relevant source path(s), API(s), input conditions and control flow. Explain the unsafe or misleading behavior precisely enough for another engineer to follow.

### Reproduction

Provide the smallest reliable reproduction, test case, request, configuration or code sample. Do not place secrets or private vulnerability details in a public pull request before coordinated disclosure is ready.

### Impact

Describe confidentiality, integrity, availability, authorization, compatibility or developer-safety impact. State important constraints that reduce or increase exploitability.

### Recommended remediation

Describe the expected security/API property rather than requiring one particular implementation unless necessary.

### Maintainer disposition

- Decision:
- Fix commit / PR:
- Regression test:
- Documentation update:
- Residual risk:

### Reviewer verification

- Re-tested commit:
- Result:
- Remaining concern:

---

## Final unresolved findings

List all findings not in `Fixed`, `Not reproducible`, or `Duplicate` status and explain whether they block `0.1.0` under the closure criteria in `docs/external-review-package.md`.

## Reviewer conclusion

Summarize the review without claiming that the software is vulnerability-free. Complete the separate sign-off template with the exact final reviewed commit and scope limitations.
