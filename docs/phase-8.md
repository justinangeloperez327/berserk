# Phase 8 — Consumer application

## Delivered source

Independent examples/minimal-api package with explicit path dependency, own workspace, one Rust application file, endpoint requests and setup instructions. The root workspace excludes it deliberately. A consumer-level loopback test exercises greeting, health, parameter capture, binary echo, text echo, invalid UTF-8, 204, 404, 405/Allow and HEAD length suppression. It uses the same build_app as main. All earlier framework code and tests are retained.

## Verification status

Checked five TOML manifests, standalone dependency resolution, workspace exclusion, source inventory and ZIP integrity. Reviewed new source against the public APIs. No Rust compiler or Cargo execution is available, so consumer compilation, dependency resolution by Cargo, loopback tests and runtime behavior remain unverified. This phase's deliverables are prepared, but its original completion gate (verified working consumer) is NOT passed. Do not describe this as a tested working release.

## Next work

Phase 9: middleware and state. The consumer verification gate remains outstanding and must be resolved in a Rust-enabled environment before claiming runtime correctness. Publication remains with the user.
