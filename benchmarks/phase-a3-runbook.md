# Phase A3: paired HTTP baseline

This is a **baseline collection procedure**, not a published performance result.
Build both frameworks in release/optimized mode from pinned revisions on the
same dedicated host. Match worker/thread counts, keep-alive, limits, telemetry,
and exposed HTTP/1.1 routes. Both fixtures must respond `200`, body `ok`.

## Berserk fixture

```sh
cargo build --release --locked -p berserk-benchmarks --bin berserk-parity-server
BERSERK_BENCH_ADDR=127.0.0.1:3000 target/release/berserk-parity-server
```

On Windows PowerShell set `$env:BERSERK_BENCH_ADDR='127.0.0.1:3000'`
and launch `target/release/berserk-parity-server.exe`.

## Gungnir fixture

Build a dedicated release-mode Gungnir executable with one GET `/` route
returning text `ok`, listening on 127.0.0.1:8000. **This repository does not
contain or build that executable**; confirm its exact source and behavior in
Gungnir before attributing measurements to framework parity.

## Execute

From the Berserk checkout, with both servers running:

```sh
python3 benchmarks/run_parity.py --berserk-url http://127.0.0.1:3000/ \
  --gungnir-url http://127.0.0.1:8000/ --output-dir benchmark-results/paired \
  --berserk-sha YOUR_BERSERK_SHA --gungnir-sha YOUR_GUNGNIR_SHA \
  --machine "CPU model; OS; compilers; workers; build flags"
```

Five alternating runs per concurrency level are default. Raw JSON for every
run and a median summary are retained. Do not commit machine-specific result
files without their full environment metadata. Report absolute p50/p95/p99 and
requests/sec. Measurements include Python client overhead and loopback TCP.

## Important limitations

The reference fixtures must be independently checked for parity of headers,
serialization and connection behavior. A `200 ok` response alone is not
proof of identical server policies. The runner does not measure RSS, CPU
utilization, allocator behavior, or open-loop saturation; collect those
separately. Avoid ranking frameworks until this acceptance work is complete.
