# Performance baseline harness

The `Performance baseline` workflow builds this harness in release mode and collects five checked runs each of routing, JSON, SQLite query execution, and sequential TCP. Download its artifact for raw CSV, per-run peak process RSS, diagnostics, and environment metadata. A `SUCCESS` marker is written only after all 15 runs pass; artifacts without it are incomplete. Shared-runner results are observations, not a stable regression threshold.

Build/run from the repository root with a Rust toolchain. Examples:

```sh
cargo run --release -p berserk-benchmarks -- routing 10000 1000
cargo run --release -p berserk-benchmarks -- json 10000 1000
cargo run --release -p berserk-benchmarks -- query 5000 500
cargo run --release -p berserk-benchmarks -- tcp 1000 100
```

Arguments: scenario, measured iterations, warmup iterations. Successful stdout contains a CSV header and one result row. Diagnostics go to stderr. Nonzero exit or missing row invalidates the run; do not interpret a partial file as a result. Each measured operation verifies its result. CSV elapsed includes timers, sample collection and checking overhead but excludes warmup and sorting. Samples use nearest-rank percentiles. The timer can report zero for very short operations; do not interpret fine-grained differences as real without repeats.

## Scenarios

- routing: builds 101 routes once, then constructs and dispatches a parameter request per iteration. Includes request allocation and result checks, not networking.
- json: parses and serializes one fixed small mixed object. Includes allocations and exact output comparison.
- query: an in-memory SQLite table with 100 seeded rows; each sample builds and executes a bound filtered/ordered/limited query and verifies 20 returned rows. This measures query construction, binding, driver execution, and row decoding together; it is not a remote-database latency benchmark.
- tcp: one sequential client; new loopback connection for every request; default worker count, queue, polling and deadlines. Includes connect, request, response and close. It is NOT a concurrent capacity benchmark and can be dominated by connection setup and OS networking costs. Large runs may encounter ephemeral-port constraints.

## Concurrent load and soak

`benchmarks/src/bin/berserk-load.rs` is a correctness-oriented network stress harness. It intentionally avoids a fixed throughput threshold because GitHub-hosted runner capacity varies. Instead, the gate fails on crashes, hangs, accounting failures, unexpected server failures, rejected steady-state work, client errors during steady/soak traffic, or failure to reject work when the bounded overload queue is saturated.

Run the short stress suite:

```sh
cargo run --release -p berserk-benchmarks --bin berserk-load -- smoke 1000 16
```

The smoke suite records:

- `concurrent_steady`: 1,000 loopback requests across 16 client threads; all work must complete without rejection or failure.
- `overload_bounded_queue`: a one-worker, two-entry queue is deliberately saturated; the server must complete some work and reject some work rather than grow an unbounded queue.
- `shutdown_under_load`: shutdown is requested while clients are active; every accepted connection must end as completed, failed, or explicitly rejected before the server exits.

Run a time-based soak:

```sh
cargo run --release -p berserk-benchmarks --bin berserk-load -- soak 300 16
```

Arguments are duration in seconds and client concurrency. The soak requires zero client errors, server failures, and queue rejections, and exact attempted/completed accounting. The harness accepts durations up to one hour.

The `Concurrent load` workflow runs the smoke suite plus a short 20-second soak on every pull request. Manual runs default to a 5-minute prolonged soak, and the scheduled main-branch run uses 15 minutes. Raw CSV and GNU `time -v` peak-RSS evidence are retained for 30 days. Throughput and p50/p95/p99/max latency are evidence for comparison, not absolute CI thresholds.

## Reproducible collection

1. First establish passing builds and correctness tests for the workspace and standalone consumer. These are covered by the main CI workflow.
2. Record source revision or archive hash, date, Rust version, OS/kernel, CPU, RAM, power mode, build profile and server settings.
3. Use release mode, stable machine conditions and identical parameters. Avoid unrelated foreground loads.
4. Run each scenario at least five times. Retain individual CSV rows; compare medians of run-level throughput and latency, plus their spread. Never average percentiles into an aggregate percentile.
5. Memory: measure the already-built executable with your OS's process resource monitor. On Linux, `/usr/bin/time -v target/release/berserk-benchmarks routing 10000 1000` reports maximum resident set size separately from stdout. Do not time Cargo when measuring process memory. RSS includes the harness, samples and runtime; it is not framework-only memory. Windows can use process monitoring, but peak working-set definitions differ from Linux RSS.
6. Archive environment notes and raw output together. The collection workflows record peak process RSS; they do not measure framework-only memory or normalize CPU utilization.

## Regression review

Compare the same scenario, machine, profile, settings and iteration counts. A repeatable throughput decrease or latency increase exceeding both 10% and observed run-to-run variation is a review trigger, not an automatic failure. Investigate correctness, changed workloads and measurement noise first. Do not compare these numbers directly to Laravel, Express or other frameworks without equivalent endpoints, networking, payloads and database work.

## Comparing retained baselines

After collecting two result directories on the same machine/settings, run:

```sh
python3 benchmarks/compare.py baseline-results current-results
```

The tool compares medians of the five run-level measurements and marks a scenario for review at a 10% throughput decrease or p95/p99 latency increase. A review marker is evidence to investigate, not a pass/fail verdict. Environment mismatch or run-to-run noise invalidates simplistic conclusions.

## Remaining measurements

Keep-alive mixes, large bodies, streaming, prolonged slow clients, CPU profiling, open-loop arrival-rate testing and statistically controlled cross-version capacity comparisons are not covered. The concurrent harness uses closed-loop clients and therefore must not be interpreted as an open-loop saturation model.

## Automated sequential collection on Linux

```sh
cargo build --locked --release -p berserk-benchmarks
LC_ALL=C python3 benchmarks/collect.py
```

Requires Python 3 and GNU `/usr/bin/time`. Run from the repository root. The collector refuses an existing output directory to prevent mixing runs. An optional first argument selects a fresh output directory. CI retains artifacts for 30 days; download evidence before expiry. Each TCP run uses 100 measured requests and 10 warmups; routing and JSON use 10,000 measured operations and 1,000 warmups.
