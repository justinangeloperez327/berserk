from pathlib import Path
import subprocess

PACKAGE_NAMES = [
    "core",
    "validation",
    "database",
    "auth",
    "openapi",
    "cache",
    "storage",
    "events",
    "jobs",
    "client",
    "notifications",
    "cli",
    "testing",
]

REPLACEMENTS = []
for name in PACKAGE_NAMES:
    REPLACEMENTS.append((f"framework-{name}", f"berserk-{name}"))
    REPLACEMENTS.append((f"framework_{name}", f"berserk_{name}"))
REPLACEMENTS.extend(
    [
        ("framework-benchmarks", "berserk-benchmarks"),
        ("framework_benchmarks", "berserk_benchmarks"),
    ]
)

TEXT_SUFFIXES = {".rs", ".toml", ".md", ".yml", ".yaml", ".txt"}
SKIP = {
    "Cargo.lock",
    ".github/workflows/package-identity-migration.yml",
    "scripts/package_identity_migration.py",
}

tracked = subprocess.check_output(["git", "ls-files", "-z"]).decode().split("\0")
for raw in tracked:
    if not raw or raw in SKIP:
        continue
    path = Path(raw)
    if path.suffix not in TEXT_SUFFIXES and path.name != "Cargo.toml":
        continue
    try:
        text = path.read_text()
    except UnicodeDecodeError:
        continue
    updated = text
    for old, new in REPLACEMENTS:
        updated = updated.replace(old, new)
    if path.name == "Cargo.toml":
        updated = updated.replace("0.0.0", "0.1.0")
    if updated != text:
        path.write_text(updated)

root = Path("Cargo.toml")
text = root.read_text()
marker = 'rust-version = "1.88"\nlicense = "MIT"\npublish = false\n'
replacement = (
    'rust-version = "1.88"\n'
    'authors = ["Justin Angelo Perez"]\n'
    'license = "MIT"\n'
    'repository = "https://github.com/justinangeloperez327/berserk"\n'
    'readme = "README.md"\n'
    'publish = false\n'
)
if marker not in text:
    raise SystemExit("workspace package metadata marker not found")
root.write_text(text.replace(marker, replacement))

descriptions = {
    "crates/core/Cargo.toml": "Core configuration, state, lifecycle, and shared foundations for Berserk.",
    "crates/validation/Cargo.toml": "Validation and sanitization contracts for Berserk.",
    "crates/database/Cargo.toml": "Database abstractions, query building, migrations, and SQL drivers for Berserk.",
    "crates/claw/Cargo.toml": "Claw ORM models, typed queries, persistence, and relationships for Berserk.",
    "crates/auth/Cargo.toml": "Authentication, password, session, and authorization contracts for Berserk.",
    "crates/openapi/Cargo.toml": "OpenAPI document generation for Berserk applications.",
    "crates/cache/Cargo.toml": "Cache contracts and in-memory caching for Berserk.",
    "crates/storage/Cargo.toml": "Storage contracts and local or memory storage for Berserk.",
    "crates/events/Cargo.toml": "Typed application event dispatch for Berserk.",
    "crates/jobs/Cargo.toml": "Bounded background jobs, retries, failures, and scheduling for Berserk.",
    "crates/client/Cargo.toml": "Outbound HTTP client contracts for Berserk.",
    "crates/notifications/Cargo.toml": "Mail and webhook notification contracts for Berserk.",
    "crates/cli/Cargo.toml": "Command-line tooling and generators for Berserk applications.",
    "crates/testing/Cargo.toml": "Testing helpers, fakes, recorders, and workspaces for Berserk.",
    "crates/framework/Cargo.toml": "Berserk web framework for secure, maintainable Rust applications.",
}
for manifest, description in descriptions.items():
    path = Path(manifest)
    text = path.read_text()
    marker = "license.workspace = true\npublish.workspace = true\n"
    replacement = (
        "license.workspace = true\n"
        "authors.workspace = true\n"
        "repository.workspace = true\n"
        "readme.workspace = true\n"
        f'description = "{description}"\n'
        "publish = true\n"
    )
    if marker not in text:
        raise SystemExit(f"publish metadata marker not found in {manifest}")
    path.write_text(text.replace(marker, replacement, 1))

for manifest in ["examples/foundation/Cargo.toml", "benchmarks/Cargo.toml"]:
    path = Path(manifest)
    text = path.read_text()
    if "publish.workspace = true" not in text:
        raise SystemExit(f"non-publishable marker not found in {manifest}")
    path.write_text(text.replace("publish.workspace = true", "publish = false", 1))

Path("LICENSE").write_text(
    """MIT License

Copyright (c) 2026 Justin Angelo Perez

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""
)
old_license = Path("LICENCE")
if old_license.exists():
    old_license.unlink()

checklist = Path("docs/release-checklist.md")
text = checklist.read_text()
text = text.replace(
    "- [ ] Select final project and package names; confirm registry and repository availability.",
    "- [ ] Confirm crates.io availability for the final `berserk`, `berserk-*`, and `claw-orm` package names.",
)
text = text.replace(
    "- [ ] Select and add a license; update every package's license/repository metadata.",
    "- [x] Select and add the MIT license; update publishable package license/repository metadata.",
)
text = text.replace(
    "- [ ] Replace version `0.0.0` and deliberately set package publication flags.",
    "- [x] Set the initial package version to `0.1.0` and explicitly mark library/CLI crates publishable while examples and benchmarks remain private.",
)
text = text.replace(
    "- [ ] Pass PostgreSQL, MySQL, and SQLite contract and migration tests against real databases.",
    "- [x] Pass PostgreSQL, MySQL, and SQLite contract and migration tests against real databases.",
)
text = text.replace(
    "- [ ] Fuzz all untrusted parsers and run server load, overload, shutdown, and soak tests.",
    "- [x] Fuzz the current JSON, HTTP-value, route-registration, and multipart input boundaries.\n- [ ] Run concurrent server load, overload, shutdown-under-load, and prolonged soak tests.",
)
text = text.replace(
    "- [ ] Record reproducible latency, throughput, memory, and environment data.",
    "- [x] Record reproducible sequential latency, throughput, process RSS, and environment data; retain raw evidence.",
)
checklist.write_text(text)

Path(".github/workflows/package.yml").write_text(
    """name: Package

on:
  pull_request:
  workflow_dispatch:

permissions:
  contents: read

jobs:
  package:
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@stable
      - name: Verify release package identities
        shell: bash
        run: |
          set -euo pipefail
          test ! -e LICENCE
          test -s LICENSE
          ! git grep -n -E 'framework-(core|validation|database|auth|openapi|cache|storage|events|jobs|client|notifications|cli|testing|benchmarks)' -- ':!Cargo.lock'
          ! git grep -n -E 'framework_(core|validation|database|auth|openapi|cache|storage|events|jobs|client|notifications|cli|testing|benchmarks)' -- ':!Cargo.lock'
          cargo metadata --locked --no-deps --format-version 1 > /tmp/metadata.json
          python3 - <<'CHECK'
          import json
          data = json.load(open('/tmp/metadata.json'))
          expected = {
              'berserk', 'berserk-core', 'berserk-validation', 'berserk-database',
              'claw-orm', 'berserk-auth', 'berserk-openapi', 'berserk-cache',
              'berserk-storage', 'berserk-events', 'berserk-jobs', 'berserk-client',
              'berserk-notifications', 'berserk-cli', 'berserk-testing',
          }
          packages = {p['name']: p for p in data['packages']}
          missing = expected - packages.keys()
          if missing:
              raise SystemExit(f'missing expected packages: {sorted(missing)}')
          for name in expected:
              package = packages[name]
              if package['version'] != '0.1.0':
                  raise SystemExit(f'{name}: expected version 0.1.0, got {package["version"]}')
              if package.get('publish') == []:
                  raise SystemExit(f'{name}: publication is disabled')
              if package.get('license') != 'MIT':
                  raise SystemExit(f'{name}: expected MIT license metadata')
              if package.get('repository') != 'https://github.com/justinangeloperez327/berserk':
                  raise SystemExit(f'{name}: repository metadata mismatch')
          for name in {'foundation-example', 'berserk-benchmarks'}:
              package = packages[name]
              if package.get('publish') != []:
                  raise SystemExit(f'{name}: must remain non-publishable')
          CHECK
      - name: Inspect package contents
        shell: bash
        run: |
          set -euo pipefail
          mkdir -p package-lists
          packages=(
            berserk-core berserk-validation berserk-database claw-orm berserk-auth
            berserk-openapi berserk-cache berserk-storage berserk-events berserk-jobs
            berserk-client berserk-notifications berserk-cli berserk-testing berserk
          )
          for package in "${packages[@]}"; do
            cargo package --locked -p "$package" --list > "package-lists/$package.txt"
            if grep -E '(^|/)(target|benchmark-results|\.git)(/|$)|(^|/)\.env($|\.)|\.(pem|key)$|credentials' "package-lists/$package.txt"; then
              echo "forbidden packaged path detected in $package" >&2
              exit 1
            fi
            cargo package --locked -p "$package" --no-verify --allow-dirty
          done
      - name: Retain package file lists
        uses: actions/upload-artifact@v4
        with:
          name: package-file-lists-${{ github.sha }}
          path: package-lists/
          if-no-files-found: error
          retention-days: 30
"""
)
