# Compatibility and versioning

Berserk v1.0.0 is the first stable API baseline. Packages use semantic versioning: breaking public API changes require a new major release.

## Toolchain and platform support

Rust 1.88 is the minimum supported Rust version. The tested operating systems, SQL database versions, and optional components are documented in the [support policy](support-policy.md).

## Application APIs

Use `App::new()` to create an application and `app.route()` to register routes. Request context, authentication, authorization, and application services are accessed through explicit application or request APIs. Query builders construct operations; terminal methods execute them.

A stable public API does not imply identical implementations across every feature, runtime, or database. Enable the Cargo features for the components you use and review the [known limitations](known-limitations.md).

## Upgrading

Consult [CHANGELOG.md](../CHANGELOG.md) when changing versions and run your application tests after upgrading. The [public API reference](public-api.md) documents the current interfaces and the [support policy](support-policy.md) describes the tested environments.
