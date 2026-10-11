#!/usr/bin/env python3
"""Check tracked source syntax, local documentation links, and build hygiene."""
from __future__ import annotations

import ast
from pathlib import Path
import re
import subprocess
import sys
import tomllib
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    tracked = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT)
    paths = [ROOT / name.decode() for name in tracked.split(b"\0") if name]
    errors: list[str] = []
    for path in paths:
        relative = path.relative_to(ROOT)
        if "target" in relative.parts[:-1]:
            errors.append(f"{relative}: tracked Cargo build output")
        try:
            if path.suffix == ".py":
                ast.parse(path.read_text(), filename=str(relative))
            elif path.suffix == ".toml" or path.name == "Cargo.lock":
                tomllib.loads(path.read_text())
            elif path.suffix == ".md":
                text = re.sub(r"^```[^\n]*\n.*?^```[^\n]*$", "", path.read_text(), flags=re.M | re.S)
                for destination in re.findall(r"!?\[[^\]]*\]\(([^)]+)\)", text):
                    parts = destination.split(maxsplit=1)
                    if not parts:
                        continue
                    destination = parts[0].strip("<>")
                    parsed = urlsplit(destination)
                    if parsed.scheme or parsed.netloc or not parsed.path:
                        continue
                    target = ROOT / parsed.path.lstrip("/") if parsed.path.startswith("/") else path.parent / parsed.path
                    if not target.exists():
                        errors.append(f"{relative}: missing local link {destination}")
        except (OSError, SyntaxError, ValueError) as error:
            errors.append(f"{relative}: {error}")
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print(f"Verified {len(paths)} tracked files: source syntax, local links, and build hygiene.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
