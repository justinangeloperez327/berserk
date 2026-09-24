"""Compare two benchmark result directories without pretending noisy CI is a hard gate."""
import csv
import json
import statistics
import sys
from pathlib import Path

SCENARIOS = ("routing", "json", "query", "tcp")

def rows(root, scenario):
    values = []
    for path in sorted(Path(root).glob(f"{scenario}-*.csv")):
        with path.open(newline="") as handle:
            row = next(csv.DictReader(handle))
        values.append({
            "ops": float(row["operations_per_second"]),
            "p50": int(row["p50_ns"]),
            "p95": int(row["p95_ns"]),
            "p99": int(row["p99_ns"]),
        })
    if len(values) != 5:
        raise ValueError(f"{root}: expected five {scenario} runs, found {len(values)}")
    return values

def median(values, key):
    return statistics.median(item[key] for item in values)

def delta(current, baseline):
    return ((current - baseline) / baseline) * 100.0

def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: compare.py BASELINE CURRENT")
    report = {}
    for scenario in SCENARIOS:
        base, current = rows(sys.argv[1], scenario), rows(sys.argv[2], scenario)
        base_ops, current_ops = median(base, "ops"), median(current, "ops")
        entry = {
            "baseline_median_ops": base_ops,
            "current_median_ops": current_ops,
            "throughput_delta_percent": delta(current_ops, base_ops),
        }
        for key in ("p50", "p95", "p99"):
            b, n = median(base, key), median(current, key)
            entry[f"baseline_median_{key}_ns"] = b
            entry[f"current_median_{key}_ns"] = n
            entry[f"{key}_delta_percent"] = delta(n, b)
        entry["review"] = (
            entry["throughput_delta_percent"] <= -10.0
            or entry["p95_delta_percent"] >= 10.0
            or entry["p99_delta_percent"] >= 10.0
        )
        report[scenario] = entry
    print(json.dumps(report, indent=2))

if __name__ == "__main__":
    main()
