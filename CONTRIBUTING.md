# Contributing

This snapshot is not yet accepting a public contribution workflow. Before proposing changes, preserve these project rules:

- Laravel-inspired conventions, Express-style structural freedom, Rust-style explicit errors.
- No hidden database execution or implicit relationship loading.
- Bounded external input and bounded background work.
- No secrets in logs, diagnostics, generated fixtures, or default `Debug` output.
- No `unsafe` code without an explicit project-level policy change and review.
- New behavior requires focused tests and user-facing documentation.

## Local quality gate

Run these commands from the workspace root:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
```

Then run the independent consumer:

```sh
cargo test --manifest-path examples/minimal-api/Cargo.toml
```

Driver changes also require live integration tests against each affected database. Never commit credentials or generated secrets.
