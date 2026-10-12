#!/usr/bin/env python3
"""Read-only preflight for an unpublished Berserk v1.0.0 candidate."""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import itertools
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import urllib.error
import urllib.request

from release_plan import build_plan, cargo_metadata

ROOT = Path(__file__).resolve().parents[1]
USER_AGENT = "Berserk-release-readiness (https://github.com/justinangeloperez327/berserk)"


def validate_soak_seconds(value: str) -> int:
    if not re.fullmatch(r"[0-9]+", value):
        raise ValueError("soak duration must be an integer number of seconds")
    duration = int(value)
    if not 300 <= duration <= 900:
        raise ValueError("release soak duration must be between 300 and 900 seconds")
    return duration


def verify_source(expected_commit: str) -> str:
    if not re.fullmatch(r"[0-9a-f]{40}", expected_commit):
        raise ValueError("expected source must be a full Git commit SHA")
    actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    if actual != expected_commit:
        raise ValueError(f"checked out {actual}, expected {expected_commit}")
    subprocess.run(["git", "diff", "--exit-code", "HEAD", "--"], cwd=ROOT, check=True)
    untracked = subprocess.check_output(
        ["git", "ls-files", "--others", "--exclude-standard"], cwd=ROOT, text=True
    ).strip()
    if untracked:
        raise ValueError(f"candidate contains untracked files: {untracked}")
    return actual


def spelling_variants(name: str) -> list[str]:
    # crates.io treats hyphens and underscores as equivalent. Observe every
    # spelling in this small publication graph rather than relying on a 404
    # for only the preferred spelling.
    parts = [("-", "_") if char in "-_" else (char,) for char in name.lower()]
    return sorted({"".join(chars) for chars in itertools.product(*parts)})


def observe_registry_name(spelling: str) -> dict[str, object]:
    url = f"https://crates.io/api/v1/crates/{spelling}"
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "application/json"})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            if response.status != 200:
                raise ValueError(f"unexpected registry status for {spelling}: {response.status}")
            data = json.load(response)
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return {"spelling": spelling, "url": url, "status": "not_registered"}
        raise ValueError(f"registry lookup failed for {spelling}: HTTP {error.code}") from error
    except (OSError, urllib.error.URLError, json.JSONDecodeError) as error:
        raise ValueError(f"registry lookup failed for {spelling}: {error}") from error
    crate = data.get("crate") if isinstance(data, dict) else None
    if not isinstance(crate, dict) or not isinstance(crate.get("id"), str):
        raise ValueError(f"malformed registry response for {spelling}")
    return {
        "spelling": spelling,
        "url": url,
        "status": "registered",
        "crate": crate["id"],
        "repository": crate.get("repository"),
    }


def check_registry_names(names: list[str]) -> dict[str, object]:
    spellings = sorted({spelling for name in names for spelling in spelling_variants(name)})
    with ThreadPoolExecutor(max_workers=4) as executor:
        observations = list(executor.map(observe_registry_name, spellings))
    occupied = [item for item in observations if item["status"] == "registered"]
    if occupied:
        details = ", ".join(f"{item['spelling']} ({item['crate']})" for item in occupied)
        raise ValueError(f"first-publication package names are already registered: {details}")
    return {
        "checked": True,
        "checked_at": datetime.now(timezone.utc).isoformat(),
        "observations": observations,
        "scope": "registry visibility only; names are not reserved and publisher permissions are not verified",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expected-commit", default=os.environ.get("GITHUB_SHA"), required=not os.environ.get("GITHUB_SHA"))
    parser.add_argument("--soak-seconds", default="300")
    parser.add_argument("--check-registry", action="store_true")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--github-output", type=Path)
    args = parser.parse_args()
    os.chdir(ROOT)
    try:
        duration = validate_soak_seconds(args.soak_seconds)
        source = verify_source(args.expected_commit)
        plan = build_plan(cargo_metadata(), "1.0.0")
        registry = check_registry_names(plan["publish_order"]) if args.check_registry else {"checked": False}
        evidence = {
            "schema_version": 1,
            "artifact_model": "release-candidate-preflight",
            "source_commit": source,
            "version": plan["version"],
            "package_count": plan["package_count"],
            "publish_order": plan["publish_order"],
            "soak_seconds": duration,
            "registry": registry,
        }
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        if args.github_output:
            with args.github_output.open("a", encoding="utf-8") as handle:
                handle.write("candidate_preflight=" + json.dumps(evidence, separators=(",", ":")) + "\n")
                handle.write(f"soak_seconds={duration}\n")
        print(f"Candidate {source}: {plan['package_count']} packages at {plan['version']}; soak {duration}s; registry checked={registry['checked']}")
        return 0
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"release candidate error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
