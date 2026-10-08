# Gungnir ↔ Berserk parity and performance programme

This programme ports **behaviour and design improvements**, not C++ implementation
details, from Gungnir into Berserk. Existing Berserk contracts should not be
replaced merely to make the public APIs look alike. Maintain both projects'
language idioms and their existing compatibility policies.

## Current evidence and implementation priorities

Both repositories already describe routes, middleware, ORM, migrations,
validation, auth, sessions, caching, storage, events, jobs, CLI, and operational
facilities. An advertised feature is not proof of equivalent behaviour.

| Area | Parity work | Verification |
| --- | --- | --- |
| HTTP runtime | Compare HTTP/1.1 connection handling, keep-alive, deadlines, overload and graceful shutdown | Same response and concurrency; separate timeout and overload tests |
| Routing | Check typed parameters, groups, fallbacks, naming, route binding and middleware order | Common black-box route fixtures |
| ORM | Compare query semantics, eager loading, transactions and errors | Identical dataset and driver versions |
| Security | Compare sessions, CSRF, cookie attributes, headers and authorization | Negative tests and threat-focused cases |
| Production | Compare health/readiness, signal handling, limits, logging and request IDs | Integration tests under load |
| Developer tools | Compare generator naming, validation, diagnostics and docs | Golden scaffold tests |
| Network protocols | Inventory Gungnir WebSockets and optional HTTP/2/TLS against Berserk | Protocol conformance before throughput claims |

Do not port Gungnir's custom compiler into Berserk: the Rust compiler, macros,
and codegen are a different language integration strategy. Consider public
developer affordances individually instead.

## External HTTP comparison

The existing Berserk internal benchmarks are **not** comparable with Gungnir
numbers. Use the framework-neutral `cross_framework.py` client instead,
against two **already running Release builds** on the **same machine**.

Configure each app with a GET `/` route returning status 200 and the literal
body `ok` (no JSON quotes). Match worker counts, timeouts, routing,
middleware, logging, compression and HTTP protocol behaviour. Keep the server
outside the benchmark process.

Example (start each application in its own terminal first):

```sh
python3 benchmarks/cross_framework.py http://127.0.0.1:3000/ --framework berserk --requests 10000 --warmup 1000 --concurrency 16 --expected-body ok --output berserk.json
python3 benchmarks/cross_framework.py http://127.0.0.1:8000/ --framework gungnir --requests 10000 --warmup 1000 --concurrency 16 --expected-body ok --output gungnir.json
```

Default source checkouts do not guarantee those ports or response bodies.
Verify both with a normal HTTP client before collecting measurements. The
runner aborts on unexpected status, body, or transport errors rather than
recording a misleading successful throughput figure.

Run at concurrency 1, 16, 64 and 128, **five or more independent runs** each.
Alternate execution order (B/G, G/B) to reduce thermal and run-order bias.
Record CPU model, OS, revisions, compilers, build flags, server thread counts,
client version, payload sizes, peak RSS and CPU load. Compare medians and
spread; report latency p50/p95/p99 and throughput separately. Keep raw JSON.
Use a dedicated machine for reliable results, not shared CI runners.

The Python client uses a closed-loop thread pool and persistent HTTP/1.1
connections when supported. This measures **end-to-end client-visible
latency**, including Python client overhead, loopback TCP and transport.
It is not a framework-only microbenchmark or a maximum-capacity open-loop
load generator. For saturation/capacity testing, use the same external load
generator with controlled arrival rates for both frameworks.

## Implementation sequence

1. Lock down paired fixtures and external benchmark protocol (this change).
2. Compare feature contracts and create missing black-box parity tests.
3. Rewrite isolated, measured hot paths while retaining API compatibility.
4. Verify routing, HTTP errors and shutdown under load.
5. Verify database and relationship performance against identical datasets.
6. Test security and production controls before further optimization.
7. Publish per-revision performance comparisons with recorded environments.

Never claim a faster framework without comparable passing runs.
