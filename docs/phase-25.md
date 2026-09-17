# Phase 25 — Developer Tooling

Phase 25 adds optional tools that shorten ordinary Laravel-inspired workflows without making generated architecture mandatory.

## CLI

`berserk-cli` includes a `framework` binary and a reusable command API. Supported commands are:

```text
framework new demo-api
framework make:model User
framework make model User
framework make model:User
framework make:migration create_users
framework migrate
framework migrate:rollback
framework migrate:status
```

`new` creates a minimal single-file API project. It accepts only a normalized relative path whose parent already exists inside the selected generator root. Existing targets are rejected. If generation fails, cleanup is limited to the new target directory.

`make:model` validates a PascalCase Rust type, creates `src/models/<name>.rs`, and safely registers its module. The generated struct intentionally contains only an `id` field and an instruction to implement `database::Model` after the real table fields are known; the generator does not invent a schema.

`make:migration` validates snake_case names and generates the actual Phase 18 driver-aware `Migration` interface with `Vec<Statement>` in both directions. Timestamped filenames prevent ordinary collisions, while `create_new` still rejects exact duplicates.

Migration execution is not discovered from folders. The application's runner implements `MigrationExecutor`, constructs its database connection and explicit migration registry, then delegates `migrate`, rollback, or status. The standalone binary returns a clear integration error until such a runner is supplied.

The generated dependency currently uses the placeholder package name and `0.1` requirement. The user controls final naming, versioning, and publication; templates must be updated when those choices are finalized.

## Testing

`berserk-testing` is a separate development crate. `TestClient` sends owned requests through `App::handle`, exercising routing, state, and middleware without TCP. `TestResponse` supplies chainable assertions for status, successful status ranges, headers, raw bytes, UTF-8 text, and parsed JSON structure.

`FakeHttpClient` returns queued responses or errors and retains every owned outbound request. `MemoryMailTransport` is re-exported for mail assertions. `EventRecorder<E>` creates a typed listener and exposes captured events. `RecordingJob` and `JobProbe` test queue retries and attempt ordering using the real job worker contract.

`TemporaryDirectory` creates a uniquely named directory under the operating-system temporary folder and removes that exact directory on drop. Tests must not place unrelated data beneath it.

Applications add testing explicitly:

```toml
[dev-dependencies]
berserk-testing = { path = "crates/testing" }
```

The main framework does not depend on or re-export `berserk-testing`, preventing a production dependency cycle and accidental inclusion of test fakes.

## Verification status

Contract test sources cover command parsing, non-overwriting project/model generation, generated migration signatures, migration delegation, in-memory request assertions, JSON comparison, outbound request recording, event recording, job retry probes, and temporary-directory cleanup. Rust tooling is unavailable in the preparation environment, so compilation and execution remain pending.
