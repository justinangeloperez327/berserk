# Upgrade notes

## 0.1.0 release candidate

`0.1.0` is Berserk's planned first public release. There is no supported migration path from an earlier crates.io release because no Berserk package has been published yet.

These notes are for developers who have already been consuming the repository through local path or git dependencies during development.

### Package identity

The release-candidate package family uses the final Berserk names, including `berserk`, `berserk-*`, and `claw-orm`, all prepared at version `0.1.0`.

If an application still references older experimental `framework-*` package identities or workspace-local names, update those dependencies before testing against the release candidate.

### Rust version

The minimum supported Rust version is Rust 1.88. Applications using an older compiler must upgrade their toolchain before adopting `0.1.0`.

### Public API stability

Pre-release path/git consumers should not assume source compatibility with earlier repository snapshots. The documented `0.1.0` contract is the current baseline:

- application assembly is instance-based through `App`;
- routing is registered through `app.route()`;
- handler signatures declare typed route parameters, validated input, and request access;
- database and Claw functionality is feature-gated;
- `User::query()` is the canonical general query-builder entry point, with convenience predicate entry points such as `User::where_(...)`;
- request-scoped database access is explicit through `request.connection()` and `request.transaction(...)`;
- optional components are disabled by default.

Use `README.md` and `docs/public-api.md` as the source of truth for the candidate API instead of examples copied from older commits.

### Feature selection

No optional feature is enabled by default. Applications must explicitly enable the components they use, for example:

```toml
[dependencies]
berserk = { version = "0.1.0", features = ["sqlite", "claw", "auth", "openapi"] }
```

Before publication, repository consumers should use the equivalent path dependency against `crates/framework`.

### Database compatibility

Review `docs/support-policy.md` before upgrading database-backed applications. The `0.1.0` support contract is PostgreSQL 15-18, MySQL 8.4 LTS, and the bundled SQLite path used by Berserk's supported `rusqlite` dependency.

### Security and operational behavior

Applications should review `SECURITY.md` and `docs/known-limitations.md` before deployment. In particular, `0.1.0` does not provide built-in TLS termination, automatic SSRF destination policy, browser-cookie/CSRF policy, or a bounded persistent session store.

### Future upgrades

Once public releases begin, breaking changes, MSRV increases, support-policy changes, and migration guidance will be recorded in `CHANGELOG.md` and this document.
