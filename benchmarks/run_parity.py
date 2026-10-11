#!/usr/bin/env python3
"""Repeat paired HTTP runs in alternating order; both servers must already be running."""
import argparse
import json
import pathlib
import statistics
import subprocess
import sys
import time

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--berserk-url", required=True)
    p.add_argument("--gungnir-url", required=True)
    p.add_argument("--output-dir", type=pathlib.Path, required=True)
    p.add_argument("--runs", type=int, default=5)
    p.add_argument("--requests", type=int, default=10000)
    p.add_argument("--warmup", type=int, default=1000)
    p.add_argument("--concurrency", type=int, nargs="+", default=[1, 16, 64, 128])
    p.add_argument("--berserk-sha", required=True)
    p.add_argument("--gungnir-sha", required=True)
    p.add_argument("--machine", required=True, help="Machine/CPU/OS/build configuration note")
    a = p.parse_args()
    if a.runs < 2 or a.requests < 1 or a.warmup < 0 or not a.concurrency or min(a.concurrency) < 1:
        p.error("invalid repetitions, requests, warmup or concurrency")
    a.output_dir.mkdir(parents=True, exist_ok=True)
    script = pathlib.Path(__file__).with_name("cross_framework.py")
    data = []
    for c in a.concurrency:
        for i in range(a.runs):
            # Swap order on successive repetitions to reduce run-order bias.
            order = ("berserk", "gungnir") if i % 2 == 0 else ("gungnir", "berserk")
            for framework in order:
                url = a.berserk_url if framework == "berserk" else a.gungnir_url
                path = a.output_dir / f"{framework}-c{c}-run{i+1}.json"
                command = [sys.executable, str(script), url, "--framework", framework,
                           "--requests", str(a.requests), "--warmup", str(a.warmup),
                           "--concurrency", str(c), "--expected-body", "ok",
                           "--expected-status", "200", "--output", str(path)]
                subprocess.run(command, check=True, stdout=subprocess.DEVNULL)
                item = json.loads(path.read_text(encoding="utf-8"))
                if item["requests"] != a.requests:
                    raise RuntimeError(f"request mismatch in {path}")
                data.append(item)
    summary = []
    for c in a.concurrency:
        for framework in ("berserk", "gungnir"):
            records = [r for r in data if r["concurrency"] == c and r["framework"] == framework]
            summary.append({
                "framework": framework, "concurrency": c, "runs": len(records),
                "median_rps": statistics.median(r["requests_per_second"] for r in records),
                "median_p50_us": statistics.median(r["p50_us"] for r in records),
                "median_p95_us": statistics.median(r["p95_us"] for r in records),
                "median_p99_us": statistics.median(r["p99_us"] for r in records),
            })
    report = {
        "berserk_sha": a.berserk_sha, "gungnir_sha": a.gungnir_sha,
        "machine": a.machine, "timestamp_unix": time.time(),
        "caveat": "Closed-loop Python HTTP client, not framework-only throughput",
        "summary": summary,
    }
    (a.output_dir / "summary.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))

if __name__ == "__main__":
    main()
