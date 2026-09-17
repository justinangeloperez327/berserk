# Performance baseline harness

The `Performance baseline` workflow builds this harness in release mode and collects five checked runs each of routing, JSON, and sequential TCP. Download its artifact for raw CSV, per-run peak process RSS, diagnostics, and environment metadata. A `SUCCESS` marker is written only after all 15 runs pass; artifacts without it are incomplete. Shared-runner results are observations, not a stable regression threshold.

Build/run from the repository root with a Rust toolchain. Examples:

```sh
cargo run --release -p framework-benchmarks -- routing 10000 1000
cargo run --release -p framework-benchmarks -- json 10000 1000
cargo run --release -p framework-benchmarks -- tcp 1000 100
```

Arguments: scenario, measured iterations, warmup iterations. Successful stdout contains a CSV header and one result row. Diagnostics go to stderr. Nonzero exit or missing row invalidates the run; do not interpret a partial file as a result. Each measured operation verifies its result. CSV elapsed includes timers, sample collection and checking overhead but excludes warmup and sorting. Samples use nearest-rank percentiles. The timer can report zero for very short operations; do not interpret fine-grained differences as real without repeats.

## Scenarios

- routing: builds 101 routes once, then constructs and dispatches a parameter request per iteration. Includes request allocation and result checks, not networking.
- json: parses and serializes one fixed small mixed object. Includes allocations and exact output comparison.
- tcp: one sequential client; new loopback connection for every request; default worker count, queue, polling and deadlines. Includes connect, request, response and close. It is NOT a concurrent capacity benchmark and can be dominated by connection setup and OS networking costs. Large runs may encounter ephemeral-port constraints.

## Reproducible collection

1. First establish passing builds and correctness tests for the workspace and standalone consumer. These are covered by the main CI workflow.
2. Record source revision or archive hash, date, Rust version, OS/kernel, CPU, RAM, power mode, build profile and server settings.
3. Use release mode, stable machine conditions and identical parameters. Avoid unrelated foreground loads.
4. Run each scenario at least five times. Retain individual CSV rows; compare medians of run-level throughput and latency, plus their spread. Never average percentiles into an aggregate percentile.
5. Memory: measure the already-built executable with your OS's process resource monitor. On Linux, `/usr/bin/time -v target/release/framework-benchmarks routing 10000 1000` reports maximum resident set size separately from stdout. Do not time Cargo when measuring process memory. RSS includes the harness, samples (roughly 16 bytes per iteration before allocator overhead) and runtime; it is not framework-only memory. Windows can use process monitoring, but peak working-set definitions differ from Linux RSS.
6. Archive environment notes and raw output together. The collection workflow records peak process RSS, CPU inventory, and memory inventory; it does not measure CPU utilization.

## Regression review

Compare the same scenario, machine, profile, settings and iteration counts. A repeatable throughput decrease or latency increase exceeding both 10% and observed run-to-run variation is a review trigger, not an automatic failure. Investigate correctness, changed workloads and measurement noise first. Do not compare these numbers directly to Laravel, Express or other frameworks without equivalent endpoints, networking, payloads and database work.

## Remaining measurements

Concurrent closed/open-loop load, keep-alive mixes, large bodies, streaming, overload rejection counts, prolonged slow clients, memory under concurrency, CPU profiling and statistical significance are not covered by this first harness. Sequential workloads suffer coordinated omission when used to infer load behavior. Resource controls have test sources but have not been load-verified.

## Automated collection on Linux

```sh
cargo build --locked --release -p framework-benchmarks
LC_ALL=C python3 benchmarks/collect.py
```

Requires Python 3 and GNU `/usr/bin/time`. Run from the repository root. The collector refuses an existing output directory to prevent mixing runs. An optional first argument selects a fresh output directory. CI retains artifacts for 30 days; download evidence before expiry. Each TCP run uses 100 measured requests and 10 warmups; routing and JSON use 10,000 measured operations and 1,000 warmups. No load, overload, shutdown-under-load, or prolonged soak gate is claimed by this workflow.
