# Performance baseline contract

Berserk performance work starts with measurement, not optimization.

The baseline suite records four distinct workloads: in-process routing, JSON parse/encode, SQLite query construction/execution/decoding, and sequential loopback HTTP. Each workload verifies correctness during measurement so a faster incorrect result cannot become a baseline.

Collection uses release builds, warmup, five independent measured runs, raw per-run CSV, peak process RSS, revision metadata, Rust/compiler information, CPU and memory information, and server settings. Raw evidence is retained; aggregate summaries never replace it.

Comparisons are valid only when workload, revision build mode, machine class, operating environment, and parameters are materially equivalent. The comparison helper uses medians of run-level throughput and latency. A 10% movement is a review trigger rather than an automatic regression gate because shared CI runners have uncontrolled scheduling and power-state variance.

Do not average percentile values and call the result an aggregate percentile. Do not compare the in-memory SQLite scenario with remote PostgreSQL/MySQL latency. Do not present sequential new-connection TCP throughput as concurrent server capacity.

The concurrent load harness separately validates bounded overload, shutdown accounting, steady-state concurrency, and soak behavior. Its observations complement the micro/baseline harness rather than replacing it.

Group 11 establishes evidence and comparison discipline. Group 12 may optimize only after a measured bottleneck is identified, with correctness tests remaining authoritative.
