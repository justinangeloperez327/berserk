# Fuzz testing

The harnesses exercise BERSERK's untrusted JSON, HTTP value, route-pattern, and multipart boundaries.

Run one target with a nightly toolchain and `cargo-fuzz`:

```text
cargo +nightly fuzz run json
```

CI performs bounded smoke runs. Longer local or scheduled campaigns should preserve any generated crash artifacts before minimizing and converting them into regression tests.
