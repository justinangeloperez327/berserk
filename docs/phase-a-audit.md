# Phase A — Gungnir parity audit and baseline

Status: **In progress**. Source-of-truth review must use an exact commit on both
repositories. This is a triage document, not a claim that every file is reviewed.

## Group A1: tracked-file inventory

Generate a machine-readable inventory of **all tracked files**:

```sh
python3 scripts/phase_a_inventory.py --output phase-a-inventory.json
```

The inventory classifies every tracked file by directory and crate; each entry
starts with `review=unreviewed` and `decision=pending`. Manually review and
assign retain / repair / extend / rewrite / remove decisions. It intentionally
does not silently mark any file reviewed. Review generated files, tests,
manifests, CI, documentation and examples. Untracked local files are not
covered. Retain the generated inventory with the exact audited commit SHA.

## Confirmed architectural differences to investigate

| Subsystem | Berserk evidence | Gungnir comparison | Decision |
| --- | --- | --- | --- |
| HTTP server | Hyper/Tokio listener with bounded queue and worker permits | Documented accepting/draining/inactive lifecycle | Extend; test deadline-bound drain |
| Request lifetime | Application shutdown handle is an atomic flag; transport has request deadlines | Per-request cancellation tokens and cooperative stream/WebSocket lifetime | Extend; do not promise forced cancellation of blocking Rust |
| Network protocols | Built-in server documented as HTTP/1, no TLS termination | Optional native HTTP/2/TLS/WebSocket | Assess optional adapters after HTTP/1 correctness |
| Data | Claw and SQL drivers for PostgreSQL/MySQL/SQLite | Additional SQL Server and MongoDB adapters | Keep SQL API; design MongoDB as document backend |
| Browser security | Cookie and CSRF policy partially application-defined | Documented hardened cookies, CSRF and trusted proxies | Contract and adversarial test gap |
| Jobs/operations | Bounded in-process workers and instrumentation | Runtime supervision, distributed adapters and overload budgets | Extend rather than replace core |
| CLI/language | Rust macros and code generation | Custom .gnr compiler | Keep Rust native; do not port C++ compiler |

Gungnir documentation itself identifies optional-adapter and typed
mail/notification limitations; do not treat documented intentions as working
features.

## High-priority findings

### A1-001: CI quality job mutates dependencies

`.github/workflows/ci.yml` quality currently runs `cargo update`, installs
`cargo-edit`, runs `cargo upgrade --incompatible allow`, commits and tries to
push modified manifests/lockfiles **before** quality checks. This makes the
workflow non-hermetic and can validate a changed commit rather than the PR head.
It also requires write permissions. **Plan:** move upgrades to a separate
explicit maintenance workflow/PR; quality must run against the checked-out SHA
with `--locked` and read-only repository permissions.

Acceptance: the quality job contains no dependency upgrade, commit, or push
steps; it validates the exact checked-out SHA. Dependency updates have their own
controlled review process.

### A1-002: no verified cross-framework performance numbers

`benchmarks/cross_framework.py` supplies a neutral transport benchmark but
the repositories have not yet supplied matching release applications, saved
raw measurements, or a shared machine record. No relative speed claims are
supported.

Acceptance: both apps return byte-for-byte equivalent status/body with matched
protocol, keep-alive, middleware and resource limits. Five runs at each
documented concurrency setting; record throughput, p50/p95/p99, RSS, CPU, source
SHAs and machine metadata. Fail comparisons on functional mismatches.

### A1-003: HTTP lifecycle gap

Berserk's listener waits for its dispatcher to finish but does not expose
Gungnir-style per-request cancellation or a bounded drain deadline.
A keep-alive connection can affect shutdown completion.

Acceptance: add tests for idle keep-alive at shutdown, in-flight completion,
deadline expiry, timeout propagation, and exact accepted/completed/rejected
accounting before designing a new lifecycle API.

## Group A2: compatibility-contract checklist

Before rewriting each subsystem:

1. Identify the public API, feature gate, and supported platforms.
2. Identify Gungnir's *implemented* (not merely documented) behavior.
3. Write equivalent passing/negative fixtures with explicit expected outputs.
4. Record Berserk backward-compatibility effects and migration requirements.
5. Select retain / repair / extend / rewrite / remove.
6. Require evidence of correctness; require paired measurements for a
   performance-only rewrite.

## Group A3: baseline plan

Use the existing cross-framework HTTP client only for HTTP/1-equivalent work.
Keep internal parser/route/ORM microbenchmarks separate from network results.
Pin revisions and compile both frameworks in release mode; document CPU, OS,
compiler version and optimization switches. Compare medians over repeated
alternating runs and retain individual results.

Run tests at concurrency 1, 16, 64 and 128 first. Add 256/512 only when both
servers satisfy correctness and bounded-capacity contracts.

## Phase A exit gate

- Tracked file inventory reviewed and decisioned, no `unreviewed` entries.
- Scope-backed feature matrix with source and test pointers.
- Non-mutating CI and locked dependency validation.
- Matched benchmark applications and machine metadata.
- Repeated raw baseline results and a reproducibility guide.
- Approved implementation sequence for Phase B.

**Not yet met:** this document and generator initiate the audit. They do not
claim a full code audit or completed performance measurements.
