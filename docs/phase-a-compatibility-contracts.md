# Phase A2 — Compatibility contracts

Status: **specified, not verified**. This is the intended black-box contract
for feature parity, not evidence that both frameworks already pass it.
Use exact source revisions and keep each framework's public API idiomatic.

## Method

For each case, record: Berserk implementation path, Gungnir implementation
path, test location, feature flags, observed result, compatibility decision,
and any benchmark consequences. Classify as `matched`, `intentional divergence`,
`Berserk gap`, `Gungnir gap`, or `not verified`. A documentation statement
does not count as an executed test.

## HTTP/1.1 contract cases

| ID | Fixture | Expected contract | Current evidence |
| --- | --- | --- | --- |
| HTTP-01 | GET / plain text | 200, byte-identical body, valid length/framing | Berserk `tests/http_expansion.rs`; Gungnir `docs/http-runtime.md` |
| HTTP-02 | Two requests on one keep-alive socket, max=2 | Exactly two responses, last marks Connection: close | Berserk `keep_alive_is_bounded`; Gungnir runtime documentation |
| HTTP-03 | Chunked request upload | Correct decode or explicitly documented rejection; **not parity-identical** currently | Berserk accepts chunked; Gungnir security-hardening documents transfer coding rejection |
| HTTP-04 | HEAD for GET resource | No response body; representation length retained | Berserk `streaming_and_head_work_over_live_transport` |
| HTTP-05 | Streamed reply | Correct framing, no truncation, backpressure | Berserk streaming test; Gungnir HTTP streaming documentation |
| HTTP-06 | Duplicate Content-Length or Host | Reject ambiguous framing before application dispatch | Berserk adapter tests; Gungnir security hardening |
| HTTP-07 | Saturated connection budget | Bounded resources, rejection accounted, no unbounded queue | Berserk queue and load harness; Gungnir overload suite documentation |
| HTTP-08 | Saturated *active request* budget | Defined 503 + Retry-After response (target, not current Berserk behavior) | Gungnir `production-resilience.md` |
| HTTP-09 | Idle keep-alive during shutdown | Stop admission and terminate idle connection without indefinite shutdown | **Berserk gap to test** |
| HTTP-10 | In-flight request during shutdown | Drain within configured window, then cooperative cancellation | **Berserk gap to implement** |
| HTTP-11 | Request body timeout | Abort slow body and avoid handler invocation | Berserk adapter deadline implementation; needs wire-level test |
| HTTP-12 | Client disconnect during streaming | Release transport resources and signal cooperative producer | Needs matched tests |

**Important:** HTTP-03 is currently an intentional protocol divergence.
Do not weaken Berserk's chunked-body support merely to match Gungnir's core.

## Routing / middleware contract cases

| ID | Fixture | Expected contract |
| --- | --- | --- |
| ROUTE-01 | 100 static routes, tail hit | Correct handler and method |
| ROUTE-02 | Typed route parameter | Valid value extracts; invalid value does not call handler |
| ROUTE-03 | Named route + reverse URL | Deterministic path encoding |
| ROUTE-04 | Nested groups/middleware | Documented prefix, name and execution order |
| ROUTE-05 | Unknown route and fallback | Defined status and response |

## Database / ORM contract cases

| ID | Fixture | Expected contract |
| --- | --- | --- |
| DB-01 | Parameter-bound filter | Same rows, no value interpolation |
| DB-02 | Read-only transaction rollback | State preserved on error |
| DB-03 | Parent listing with eager relations | Bounded query count and same relation cardinality |
| DB-04 | Pagination with ordering | Identical records and metadata |
| DB-05 | Empty IN list | Defined no-match semantics |
| DB-06 | Unsupported backend capability | Explicit error, no silent approximation |

Use **the same SQL database and seeded rows** for DB fixtures. MongoDB is
a separate document-backend contract until Berserk implements a driver.

## Security and production contract cases

| ID | Fixture | Expected contract |
| --- | --- | --- |
| SEC-01 | Invalid Host / duplicated framing | Reject before dispatch |
| SEC-02 | Untrusted Forwarded headers | Never override socket peer identity |
| SEC-03 | CORS preflight | Explicit allow/deny and Vary semantics |
| SEC-04 | Cookie prefixes / SameSite=None | Enforce secure browser invariants |
| SEC-05 | Session renewal and CSRF | Rotation and constant-time token check |
| OPS-01 | Liveness/readiness callback fails | Fail closed with named diagnosis |
| OPS-02 | Shutdown during active queue and HTTP work | Accurate accounting, bounded/cooperative cleanup |
| OPS-03 | Retry policy on non-idempotent work | No automatic unclassified retry |

## Implementation priority and exit conditions

1. Preserve existing passing HTTP tests and add the missing shutdown tests.
2. Establish a shutdown lifecycle state contract before changing the server.
3. Differentiate connection limit from active dispatch limit; do not replace
   queue rejection with a 503 response without designing wire semantics.
4. Run existing routing and ORM suites before adding paired fixtures.
5. Only then benchmark matched HTTP and database workloads.

Phase A2 is complete when each row has **actual implementation and executed
test evidence** or an explicitly approved unsupported/divergent classification.
This document alone does not close the phase.
