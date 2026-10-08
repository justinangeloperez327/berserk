#!/usr/bin/env python3
"""External HTTP benchmark for identical Berserk and Gungnir endpoints.

Measures complete HTTP request/response latency (including client overhead).
No framework-specific code or third-party dependencies.
"""
import argparse
import concurrent.futures
import http.client
import json
import math
import platform
import statistics
import time
from urllib.parse import urlsplit


def percentile(sorted_samples, fraction):
    if not sorted_samples:
        raise ValueError("no successful samples")
    index = max(0, math.ceil(len(sorted_samples) * fraction) - 1)
    return sorted_samples[index]


def worker(target, requests, timeout, expected_status, expected_body):
    connection = None
    measurements = []
    try:
        for _ in range(requests):
            if connection is None:
                connection = http.client.HTTPConnection(target.hostname, target.port or 80, timeout=timeout)
            started = time.perf_counter_ns()
            try:
                connection.request("GET", target.path + ("?" + target.query if target.query else ""),
                                   headers={"Accept": "*/*"})
                response = connection.getresponse()
                body = response.read()
                if response.status != expected_status:
                    raise RuntimeError(f"HTTP {response.status}, expected {expected_status}")
                if expected_body is not None and body != expected_body:
                    raise RuntimeError(f"body mismatch: {body[:100]!r}")
            except Exception:
                connection.close()
                raise
            measurements.append((time.perf_counter_ns() - started) / 1000.0)
            if response.will_close:
                connection.close()
                connection = None
    finally:
        if connection is not None:
            connection.close()
    return measurements


def distribute(total, concurrency):
    return [total // concurrency + (1 if i < total % concurrency else 0)
            for i in range(concurrency)]


def run_batch(target, total, concurrency, timeout, expected_status, expected_body):
    start = time.perf_counter()
    with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
        futures = [executor.submit(worker, target, n, timeout, expected_status, expected_body)
                   for n in distribute(total, concurrency) if n]
        samples = []
        for future in futures:
            samples.extend(future.result())
    return samples, time.perf_counter() - start


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url", help="Exact endpoint, e.g. http://127.0.0.1:8000/")
    parser.add_argument("--framework", required=True, choices=("berserk", "gungnir"))
    parser.add_argument("--requests", type=int, default=10000)
    parser.add_argument("--warmup", type=int, default=1000)
    parser.add_argument("--concurrency", type=int, default=16)
    parser.add_argument("--timeout", type=float, default=5.0)
    parser.add_argument("--expected-status", type=int, default=200)
    parser.add_argument("--expected-body", default=None,
                        help="Validate exact UTF-8 body, or omit to validate status only")
    parser.add_argument("--output", help="Optional JSON output path")
    args = parser.parse_args()
    target = urlsplit(args.url)
    if target.scheme != "http" or not target.hostname or target.username or target.password or target.fragment:
        parser.error("URL must be a plain HTTP URL without credentials or fragment")
    if min(args.requests, args.concurrency) <= 0 or args.warmup < 0 or args.timeout <= 0:
        parser.error("requests/concurrency/timeout must be positive; warmup must be nonnegative")
    expected_body = args.expected_body.encode("utf-8") if args.expected_body is not None else None
    if args.warmup:
        run_batch(target, args.warmup, args.concurrency, args.timeout,
                  args.expected_status, expected_body)
    samples, seconds = run_batch(target, args.requests, args.concurrency, args.timeout,
                                 args.expected_status, expected_body)
    samples.sort()
    result = {
        "framework": args.framework,
        "url": args.url,
        "requests": len(samples),
        "concurrency": args.concurrency,
        "warmup": args.warmup,
        "elapsed_seconds": round(seconds, 6),
        "requests_per_second": round(len(samples) / seconds, 3),
        "p50_us": round(percentile(samples, .50), 3),
        "p95_us": round(percentile(samples, .95), 3),
        "p99_us": round(percentile(samples, .99), 3),
        "max_us": round(samples[-1], 3),
        "client": f"Python {platform.python_version()} http.client",
        "platform": platform.platform(),
    }
    output = json.dumps(result, indent=2)
    print(output)
    if args.output:
        with open(args.output, "w", encoding="utf-8") as file:
            file.write(output + "\n")


if __name__ == "__main__":
    main()
