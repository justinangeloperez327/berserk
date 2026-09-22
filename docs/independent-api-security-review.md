# Independent API and security review

Status: **pending**

This document is the evidence record for the final Berserk V1 framework-maturity
blocker. Internal development review, automated scanners, CI, fuzzing, and the
repository owner's approval are useful evidence but do **not** by themselves
satisfy this gate.

## Review target

The reviewer must record the exact commit SHA reviewed. If release-blocking
findings are fixed afterward, the reviewer must inspect the remediation and
update the reviewed commit/evidence before this gate is closed.

- Candidate commit: `PENDING`
- Review date: `PENDING`
- Reviewer / team: `PENDING`
- Reviewer relationship to implementation: `PENDING`
- Review result: `PENDING`

The reviewer should not be the same person/process that authored the
implementation being signed off. A separate qualified reviewer, security
engineer, assessment team, or otherwise independent review process should own
the conclusion.

## Required scope

The independent review should cover at least these boundaries:

- public exports, prelude, routing, handlers, controller contracts, state and
  inversion-of-control boundaries;
- HTTP parsing/encoding, request limits, headers, response framing, redirects,
  middleware ordering, error rendering, logging, tracing, rate limiting,
  overload and shutdown behavior;
- authentication, token/session lifecycle, password handling, principal scope,
  route abilities, policies, credential redaction and authentication failure
  behavior;
- JSON, query, multipart and FormRequest decode/sanitize/authorize/validate
  ordering;
- database statements/bindings, raw SQL escape hatches, migrations,
  transactions, Claw writes, model binding and eager relationships;
- Axe escaping, `SafeHtml`, template/include validation and production cache
  behavior;
- storage path isolation, symlink/TOCTOU boundaries, temporary writes and
  configured capacity limits;
- outbound HTTP URL/header/framing handling, HTTPS boundary and application
  SSRF responsibilities;
- cache, jobs, events, notifications and other resource-capacity or
  side-effect/idempotency boundaries;
- CLI/code generation path safety, generated application contracts and
  dependency/publication boundaries;
- diagnostics and `Debug` implementations for accidental disclosure of paths,
  headers, bodies, SQL/bindings, tokens, credentials or application data;
- documented limitations, support policy, vulnerability reporting, release
  recovery and publication claims.

## Baseline evidence to consult

The reviewer should compare implementation against:

- `SECURITY.md`
- `docs/security-review.md`
- `docs/acceptance-checks.md`
- `docs/public-api.md`
- `docs/known-limitations.md`
- `docs/support-policy.md`
- `docs/vulnerability-response.md`
- `docs/v1-maturity-gate.md`
- the foundation reference application
- the exact-commit CI/security/database/fuzz/load/package evidence available for
  the candidate

The internal review record contains historical findings and the V1 pre-review
remediation items. The independent reviewer should verify those fixes rather
than assuming them correct.

## Finding format

Record every material finding with:

| Field | Required content |
| --- | --- |
| ID | Stable identifier such as `IR-01` |
| Severity | Critical, High, Medium, Low, Documentation, or Process |
| Component | Crate/module/public contract |
| Evidence | File/API/test/reproduction |
| Impact | Concrete confidentiality, integrity, availability, or compatibility consequence |
| Resolution | Fixed, mitigated, documented/accepted, or release-blocking |
| Verification | Commit/test/evidence showing the disposition was checked |

Critical or High findings affecting the intended release configuration block
publication until fixed or explicitly resolved under the vulnerability response
process. Medium/Low findings must also have an explicit disposition; they cannot
silently disappear from the review record.

## Findings

No independent findings have been recorded yet.

## Sign-off checklist

The reviewer completes these only after reviewing the exact candidate and any
remediation:

- [ ] Reviewed the exact candidate commit recorded above.
- [ ] Reviewed the public API for accidental instability, ambiguous ownership,
      hidden execution, or unsafe defaults.
- [ ] Reviewed security-sensitive trust boundaries listed in Required scope.
- [ ] Verified SR-12 route-template log redaction.
- [ ] Verified SR-13 fallible local-storage entropy handling.
- [ ] Verified current bounded `MemorySessionStore` behavior and documentation.
- [ ] Checked that public errors and default diagnostics do not expose secrets or
      internal details.
- [ ] Checked SQL/binding separation and explicit raw-SQL boundaries.
- [ ] Checked authentication/authorization failure behavior and credential
      handling.
- [ ] Checked documented deployment limitations against implementation.
- [ ] Recorded every material finding and its disposition.
- [ ] Confirmed no unresolved release-blocking finding remains.

## Reviewer conclusion

`PENDING`

The framework-maturity checkbox in `docs/v1-maturity-gate.md` must remain
unchecked until this document contains a completed independent review against a
specific commit with all release-blocking findings resolved or explicitly
documented.
