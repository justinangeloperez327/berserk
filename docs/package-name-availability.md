# crates.io package-name availability check

Historical check date: **2026-09-17**

This file records historical evidence only. crates.io package availability is
time-sensitive and package names are not reserved by a repository check.

## Current publication graph

The current repository release planner
(`scripts/release_plan.py`) defines **18 publishable crates**:

| Package | 2026-09-17 historical evidence |
| --- | --- |
| `berserk` | No collision found in the historical check |
| `berserk-core` | No collision found in the historical check |
| `berserk-codegen` | **Not covered by the historical check** |
| `berserk-macros` | **Not covered by the historical check** |
| `berserk-validation` | No collision found in the historical check |
| `berserk-database` | No collision found in the historical check |
| `claw-orm` | No collision found in the historical check |
| `berserk-auth` | No collision found in the historical check |
| `berserk-axe` | **Not covered by the historical check** |
| `berserk-openapi` | No collision found in the historical check |
| `berserk-cache` | No collision found in the historical check |
| `berserk-storage` | No collision found in the historical check |
| `berserk-events` | No collision found in the historical check |
| `berserk-jobs` | No collision found in the historical check |
| `berserk-client` | No collision found in the historical check |
| `berserk-notifications` | No collision found in the historical check |
| `berserk-cli` | No collision found in the historical check |
| `berserk-testing` | No collision found in the historical check |

The earlier evidence therefore covered **15 of the current 18 names**. It is
not sufficient to satisfy the final V1 release gate.

## Historical method

The 2026-09-17 check used the official crates.io index and checked canonical
names plus normalization-equivalent underscore forms where relevant. At that
time, no checked name had an index collision.

That result must not be extrapolated to names added to the publication graph
later.

## Final release requirement

Immediately before the first publication:

1. derive the exact publishable package set from
   `scripts/release_plan.py --version 1.0.0`;
2. confirm it contains exactly 18 intended packages;
3. check every canonical package name against crates.io;
4. check case/normalization-equivalent collisions, including `-` versus `_`;
5. record the check date and result for all 18 names;
6. stop publication if any intended name is no longer available.

Only that fresh 18-package check can close the package-name release gate.

No crate was published, reserved, tagged, or otherwise released by the
2026-09-17 historical check.
