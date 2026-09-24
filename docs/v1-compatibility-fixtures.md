# V1 compatibility fixtures

Berserk 1.x treats the public v1.0 application surface as a compatibility contract.

The executable firewall is `scripts/verify_v1_compatibility.py`. It creates independent Cargo workspaces rather than compiling fixtures as members of the Berserk workspace. This catches changes that remain invisible to internal crate tests, including facade re-exports, feature wiring, trait signatures, and consumer-visible type changes.

## Frozen surfaces

The current fixtures compile representative contracts for:

- application assembly through `App` and `app.route()`;
- typed routes and request/response APIs;
- `FromJson` and `FormRequest`;
- the `#[derive(Model)]` model surface;
- Claw `Model`, `ModelQuery`, `Collection<T>`, and query builders;
- `IntoInsert` / `IntoUpdate`;
- `CrudController`;
- selected prelude and integration exports with auth, view, Claw, and SQLite enabled.

The fixtures intentionally use `default-features = false`. Feature-specific cases opt in only to the public components they exercise. The normal CI feature matrix remains responsible for exhaustive independent feature compilation.

## Rules for 1.x

1. Do not edit a v1.0 fixture merely to make an accidental breaking change pass.
2. Additive APIs normally do not require a fixture unless they become a documented compatibility promise.
3. A deliberate breaking change requires a new major version.
4. Bug fixes may change behavior when the previous behavior violated the documented contract, but the change must be documented.
5. Raising the MSRV is a compatibility change.
6. Future stable baselines should be added as separate fixtures rather than rewriting the v1.0 contract.

Run locally with:

```sh
python3 scripts/verify_v1_compatibility.py
```

The CI quality job runs the same command after generator verification.
