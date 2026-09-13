# Performance baseline harness

This harness has not been compiled or executed. No baseline numbers exist yet.

Build/run from the repository root with a Rust toolchain. Examples:

```sh
cargo run --release -p framework-benchmarks -- routing 10000 1000
cargo run --release -p framework-benchmarks -- json 10000 1000
cargo run --release -p framework-benchmarks -- codec 10000 1000
cargo run --release -p framework-benchmarks -- tcp 1000 100
```

Arguments: scenario, measured iterations, warmup iterations. Successful stdout contains a CSV header and one result row. Diagnostics go to stderr. Nonzero exit or missing row invalidates the run; do not interpret a partial file as a result. Each measured operation verifies its result. CSV elapsed includes timers, sample collection and checking overhead but excludes warmup and sorting. Samples use nearest-rank percentiles. The timer can report zero for very short operations; do not interpret fine-grained differences as real without repeats.

## Scenarios

- routing: builds 101 routes once, then constructs and dispatches a parameter request per iteration. Includes request allocation and result checks, not networking.
- json: parses and serializes one fixed small mixed object. Includes allocations and exact output comparison.
- codec: parses one buffered request and encodes a response with a five-byte body. No networking.
- tcp: one sequential client; new loopback connection for every request; default worker count, queue, polling and deadlines. Includes connect, request, response and close. It is NOT a concurrent capacity benchmark and can be dominated by the 5 ms accept poll and OS networking costs. Large runs may encounter ephemeral-port constraints.

## Reproducible collection

1. First establish passing builds and correctness tests for the workspace and standalone consumer. Those gates are still pending.
2. Record source revision or archive hash, date, Rust version, OS/kernel, CPU, RAM, power mode, build profile and server settings.
3. Use release mode, stable machine conditions and identical parameters. Avoid unrelated foreground loads.
4. Run each scenario at least five times. Retain individual CSV rows; compare medians of run-level throughput and latency, plus their spread. Never average percentiles into an aggregate percentile.
5. Memory: measure the already-built executable with your OS's process resource monitor. On Linux, `/usr/bin/time -v target/release/framework-benchmarks routing 10000 1000` reports maximum resident set size separately from stdout. Do not time Cargo when measuring process memory. RSS includes the harness, samples (roughly 16 bytes per iteration before allocator overhead) and runtime; it is not framework-only memory. Windows can use process monitoring, but peak working-set definitions differ from Linux RSS.
6. Archive environment notes and raw output together. No actual memory or CPU values have been collected here.

## Regression review

Compare the same scenario, machine, profile, settings and iteration counts. A repeatable throughput decrease or latency increase exceeding both 10% and observed run-to-run variation is a review trigger, not an automatic failure. Investigate correctness, changed workloads and measurement noise first. Do not compare these numbers directly to Laravel, Express or other frameworks without equivalent endpoints, networking, payloads and database work.

## Remaining measurements

Concurrent closed/open-loop load, keep-alive mixes, large bodies, streaming, overload rejection counts, prolonged slow clients, memory under concurrency, CPU profiling and statistical significance are not covered by this first harness. Sequential workloads suffer coordinated omission when used to infer load behavior. Resource controls have test sources but have not been load-verified.
