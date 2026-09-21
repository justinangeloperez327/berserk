#!/usr/bin/env python3
"""Validate Berserk's publishable workspace and derive crates.io publication order.

This tool is intentionally read-only. It never uploads, yanks, tags, or modifies a
registry. Publication remains an explicit maintainer action.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

EXPECTED_PACKAGES = [
    "berserk-core",
    "berserk-validation",
    "berserk-database",
    "claw-orm",
    "berserk-auth",
    "berserk-axe",
    "berserk-openapi",
    "berserk-cache",
    "berserk-storage",
    "berserk-events",
    "berserk-jobs",
    "berserk-client",
    "berserk-notifications",
    "berserk-cli",
    "berserk-testing",
    "berserk",
]


def cargo_metadata() -> dict[str, Any]:
    output = subprocess.check_output(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        text=True,
    )
    return json.loads(output)


def package_is_publishable(package: dict[str, Any]) -> bool:
    # Cargo metadata reports `publish = false` as an empty list. Null means
    # publication is unrestricted by the manifest.
    return package.get("publish") != []


def build_plan(metadata: dict[str, Any], expected_version: str | None) -> dict[str, Any]:
    packages_by_id = {package["id"]: package for package in metadata["packages"]}
    workspace_packages = [packages_by_id[package_id] for package_id in metadata["workspace_members"]]
    packages = {package["name"]: package for package in workspace_packages}

    missing = [name for name in EXPECTED_PACKAGES if name not in packages]
    if missing:
        raise ValueError(f"missing expected publishable packages: {', '.join(missing)}")

    disabled = [name for name in EXPECTED_PACKAGES if not package_is_publishable(packages[name])]
    if disabled:
        raise ValueError(f"publication unexpectedly disabled: {', '.join(disabled)}")

    unexpected = sorted(
        package["name"]
        for package in workspace_packages
        if package_is_publishable(package) and package["name"] not in EXPECTED_PACKAGES
    )
    if unexpected:
        raise ValueError(f"unexpected publishable workspace packages: {', '.join(unexpected)}")

    versions = {packages[name]["version"] for name in EXPECTED_PACKAGES}
    if len(versions) != 1:
        details = ", ".join(f"{name}={packages[name]['version']}" for name in EXPECTED_PACKAGES)
        raise ValueError(f"publishable packages are not version-synchronized: {details}")

    version = versions.pop()
    if expected_version is not None and version != expected_version:
        raise ValueError(f"workspace version is {version}, expected {expected_version}")

    expected_set = set(EXPECTED_PACKAGES)
    for package in workspace_packages:
        if package["version"] != version:
            raise ValueError(f"{package['name']}: workspace version must be {version}")
        for dependency in package.get("dependencies", []):
            if dependency["name"] in expected_set and dependency.get("path"):
                if dependency["req"] != f"={version}":
                    raise ValueError(
                        f"{package['name']}: internal dependency {dependency['name']} "
                        f"must require ={version}, got {dependency['req']}"
                    )
    internal_dependencies: dict[str, list[str]] = {}
    for name in EXPECTED_PACKAGES:
        dependencies = {
            dependency["name"]
            for dependency in packages[name].get("dependencies", [])
            if dependency.get("kind") != "dev" and dependency["name"] in expected_set
        }
        internal_dependencies[name] = sorted(dependencies)

    remaining = {name: set(deps) for name, deps in internal_dependencies.items()}
    publish_order: list[str] = []
    publish_layers: list[list[str]] = []

    while remaining:
        ready = sorted(name for name, dependencies in remaining.items() if not dependencies)
        if not ready:
            rendered = {name: sorted(deps) for name, deps in remaining.items()}
            raise ValueError(f"internal dependency cycle detected: {rendered}")

        publish_layers.append(ready)
        publish_order.extend(ready)
        for name in ready:
            remaining.pop(name)
        for dependencies in remaining.values():
            dependencies.difference_update(ready)

    return {
        "schema_version": 1,
        "version": version,
        "package_count": len(EXPECTED_PACKAGES),
        "packages": [
            {
                "name": name,
                "version": packages[name]["version"],
                "internal_dependencies": internal_dependencies[name],
            }
            for name in EXPECTED_PACKAGES
        ],
        "publish_layers": publish_layers,
        "publish_order": publish_order,
    }


def render_text(plan: dict[str, Any]) -> str:
    lines = [
        f"Berserk release {plan['version']}",
        f"Publishable packages: {plan['package_count']}",
        "",
        "Dependency-safe publication order:",
    ]
    for index, name in enumerate(plan["publish_order"], start=1):
        deps = next(
            package["internal_dependencies"]
            for package in plan["packages"]
            if package["name"] == name
        )
        suffix = f" (after: {', '.join(deps)})" if deps else ""
        lines.append(f"{index:2}. {name}{suffix}")
    return "\n".join(lines) + "\n"


def render_commands(plan: dict[str, Any]) -> str:
    lines = [
        "# Review every command before running it. These commands publish permanently.",
        f"# Berserk {plan['version']} — dependency-safe order",
    ]
    lines.extend(f"cargo publish --locked -p {name}" for name in plan["publish_order"])
    return "\n".join(lines) + "\n"


def render_tsv(plan: dict[str, Any]) -> str:
    package_map = {package["name"]: package for package in plan["packages"]}
    return "".join(
        f"{name}\t{package_map[name]['version']}\n" for name in plan["publish_order"]
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", help="Require this exact synchronized workspace version")
    parser.add_argument(
        "--format",
        choices=("text", "json", "commands", "tsv"),
        default="text",
        help="Output representation (default: text)",
    )
    parser.add_argument("--output", type=Path, help="Write output to this path instead of stdout")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        plan = build_plan(cargo_metadata(), args.version)
    except (subprocess.CalledProcessError, json.JSONDecodeError, KeyError, ValueError) as error:
        print(f"release plan error: {error}", file=sys.stderr)
        return 1

    if args.format == "json":
        rendered = json.dumps(plan, indent=2, sort_keys=True) + "\n"
    elif args.format == "commands":
        rendered = render_commands(plan)
    elif args.format == "tsv":
        rendered = render_tsv(plan)
    else:
        rendered = render_text(plan)

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
