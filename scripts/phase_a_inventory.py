#!/usr/bin/env python3
"""Generate a deterministic tracked-file inventory for Berserk Phase A.

This is classification evidence, not a substitute for human source review.
Uses only the standard library and never changes source files.
"""
import argparse
import json
import pathlib
import subprocess
from collections import Counter

CATEGORIES = {
    "crates/": "framework-source",
    "benchmarks/": "benchmark",
    "examples/": "example",
    "docs/": "documentation",
    ".github/workflows/": "ci-workflow",
    "scripts/": "tooling",
    "tests/": "integration-tests",
}

def tracked_files(root):
    result = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, capture_output=True, check=True
    )
    return sorted(
        pathlib.PurePosixPath(raw.decode("utf-8", errors="surrogateescape")).as_posix()
        for raw in result.stdout.split(b"\0") if raw
    )

def group(path):
    for prefix, value in CATEGORIES.items():
        if path.startswith(prefix):
            return value
    return "repository-other"

def crate(path):
    bits = path.split("/")
    return bits[1] if len(bits) > 2 and bits[0] == "crates" else None

def inventory(root):
    paths = tracked_files(root)
    entries = []
    for path in paths:
        entry = {
            "path": path,
            "group": group(path),
            "extension": pathlib.PurePosixPath(path).suffix or "(none)",
            "review": "unreviewed",
            "decision": "pending",
        }
        if crate(path):
            entry["crate"] = crate(path)
        entries.append(entry)
    categories = Counter(item["group"] for item in entries)
    crates = Counter(item["crate"] for item in entries if "crate" in item)
    return {
        "schema_version": 1,
        "source": "git ls-files",
        "tracked_file_count": len(entries),
        "by_group": dict(sorted(categories.items())),
        "by_crate": dict(sorted(crates.items())),
        "files": entries,
    }

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path(__file__).resolve().parent.parent)
    parser.add_argument("--output", type=pathlib.Path, help="Write JSON to a file (otherwise stdout)")
    args = parser.parse_args()
    root = args.root.resolve()
    data = json.dumps(inventory(root), indent=2, ensure_ascii=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(data, encoding="utf-8")
    else:
        print(data, end="")

if __name__ == "__main__":
    main()
