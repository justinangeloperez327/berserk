"""Collect checked release benchmarks on Linux; build the binary first."""
import csv
import io
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import sys


def main():
    output = Path(sys.argv[1] if len(sys.argv) > 1 else "benchmark-results")
    output.mkdir(parents=True, exist_ok=False)
    binary = Path("target/release/framework-benchmarks").resolve()
    environment = {
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "rust": subprocess.check_output(["rustc", "-Vv"], text=True),
        "platform": platform.platform(),
        "cpu_count": os.cpu_count(),
        "cpu": Path("/proc/cpuinfo").read_text(),
        "memory": Path("/proc/meminfo").read_text(),
        "profile": "release",
        "repeats": 5,
        "server_settings": "App::bind defaults; sequential loopback; new connection per request",
        "limitations": "Shared runner, uncontrolled power mode; RSS includes the whole benchmark process. No capacity or soak claim.",
    }
    (output / "environment.json").write_text(json.dumps(environment, indent=2) + "\n")
    expected_names = {"routing": "routing_101_routes", "json": "json_parse_encode", "tcp": "tcp_sequential_new_connection"}
    for scenario, iterations, warmup in [("routing", 10000, 1000), ("json", 10000, 1000), ("tcp", 100, 10)]:
        for repeat in range(1, 6):
            prefix = output / f"{scenario}-{repeat}"
            result = subprocess.run(
                ["/usr/bin/time", "-v", "-o", str(prefix) + ".memory.txt", str(binary), scenario, str(iterations), str(warmup)],
                text=True, capture_output=True, timeout=180, check=False,
            )
            Path(str(prefix) + ".stderr.txt").write_text(result.stderr)
            Path(str(prefix) + ".csv").write_text(result.stdout)
            result.check_returncode()
            rows = list(csv.DictReader(io.StringIO(result.stdout)))
            if len(rows) != 1:
                raise ValueError(f"{prefix}: expected exactly one result row")
            row = rows[0]
            if row["scenario"] != expected_names[scenario] or int(row["iterations"]) != iterations or int(row["warmup"]) != warmup:
                raise ValueError(f"{prefix}: unexpected workload")
            for field in ["elapsed_seconds", "operations_per_second"]:
                value = float(row[field])
                if not math.isfinite(value) or value <= 0:
                    raise ValueError(f"{prefix}: invalid {field}")
            latency = [int(row[key]) for key in ["p50_ns", "p95_ns", "p99_ns", "max_ns"]]
            if latency[0] < 0 or latency != sorted(latency):
                raise ValueError(f"{prefix}: invalid latency ordering")
            memory = Path(str(prefix) + ".memory.txt").read_text()
            if "Maximum resident set size (kbytes):" not in memory:
                raise ValueError(f"{prefix}: missing peak RSS")
    (output / "SUCCESS").write_text("All 15 checked benchmark runs completed.\n")


if __name__ == "__main__":
    main()
