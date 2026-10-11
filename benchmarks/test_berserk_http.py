#!/usr/bin/env python3
"""Smoke-test the actual Berserk HTTP benchmark fixture and client."""
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "benchmarks"))
import cross_framework as benchmark  # noqa: E402


class BenchmarkContracts(unittest.TestCase):
    def test_distribution_preserves_request_count(self):
        for requests, concurrency in [(1, 16), (20, 4), (101, 16)]:
            buckets = benchmark.distribute(requests, concurrency)
            self.assertEqual(sum(buckets), requests)
            self.assertLessEqual(max(buckets) - min(buckets), 1)

    def test_percentile_nearest_rank(self):
        data = list(range(1, 101))
        self.assertEqual(benchmark.percentile(data, 0.5), 50)
        self.assertEqual(benchmark.percentile(data, 0.95), 95)
        self.assertEqual(benchmark.percentile(data, 0.99), 99)

    def test_live_http_fixture(self):
        binary = ROOT / "target" / "release" / (
            "berserk-parity-server.exe" if os.name == "nt" else "berserk-parity-server"
        )
        self.assertTrue(binary.exists(), f"missing release fixture {binary}")
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        env = dict(os.environ, BERSERK_BENCH_ADDR=f"127.0.0.1:{port}")
        process = subprocess.Popen([str(binary)], cwd=ROOT, env=env,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        try:
            for _ in range(100):
                if process.poll() is not None:
                    raise AssertionError(f"fixture exited with {process.returncode}")
                try:
                    with socket.create_connection(("127.0.0.1", port), timeout=0.1):
                        break
                except OSError:
                    time.sleep(0.05)
            else:
                self.fail("fixture did not listen")
            from urllib.parse import urlsplit
            samples, elapsed = benchmark.run_batch(
                urlsplit(f"http://127.0.0.1:{port}/"), 32, 4, 2.0, 200, b"ok"
            )
            self.assertEqual(len(samples), 32)
            self.assertGreater(elapsed, 0)
            self.assertTrue(all(value >= 0 for value in samples))
        finally:
            process.terminate()
            try:
                process.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.communicate(timeout=5)


if __name__ == "__main__":
    unittest.main()
