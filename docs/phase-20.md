# Phase 20 — API resources and OpenAPI documentation

Phase 20 separates public API representations from internal models and generates an OpenAPI 3.1 document from explicit endpoint contracts.

## API resources

```rust
impl ApiResource for User {
    fn to_resource(&self) -> Json {
        Json::Object(BTreeMap::from([
            ("id".into(), self.id.into()),
            ("name".into(), self.name.clone().into()),
        ]))
    }
}

app.get("/users/{id}", |_request| Resource::new(user))?;
```

`Resource<T>` produces a `{ "data": ... }` response. `ResourceCollection<T>` produces a data array with optional `links` and `meta`. Only fields selected by `ApiResource::to_resource` become public; database models are never serialized automatically.

## OpenAPI contracts

The `framework-openapi` crate builds OpenAPI 3.1 documents without depending on the HTTP server:

```rust
let mut document = OpenApi::new(Info::new("Users API", "1.0.0")?);
document.schema("User", user_schema())?;
document.security_scheme("bearerAuth", SecurityScheme::bearer(Some("opaque")))?;

let show = Operation::new("showUser")?
    .parameter(Parameter::new("id", ParameterLocation::Path, Schema::integer())?)?
    .response("200", ApiResponse::new("User found")?.json(SchemaRef::named("User")?))?
    .secured_by("bearerAuth", std::iter::empty::<String>())?;
```

Schemas support objects, arrays, strings, integers, numbers, booleans, properties, required fields, formats, string enums, descriptions, examples, inline schemas, and reusable component references. Operations support parameters, JSON request bodies, response contracts, tags, descriptions, and security requirements.

Duplicate schemas, security schemes, operations, operation IDs, parameters, request bodies, responses, properties, and enum values fail during setup. Path parameters must exactly match `{name}` segments in declaration order. Export fails for unresolved schema or security references.

## Runtime synchronization

With the framework's optional `openapi` feature, runtime registration and documentation can share one setup action:

```rust
app.documented_route(
    &mut document,
    HttpMethod::Get,
    "/users/{id}",
    show_operation,
    show_user,
)?;
```

The document is staged and validated first. The runtime route is then registered; the staged document replaces the original only if both operations succeed. This prevents a documented route from being committed without its runtime route.

`document.to_json()` returns formatted OpenAPI JSON. An application can precompute it during startup and expose it with an explicit `application/json` response route.

## Deferred scope

Derive/attribute macros, automatic Rust-type reflection, Swagger UI, ReDoc, YAML output, callbacks, webhooks, discriminators, all JSON Schema keywords, OAuth flow builders, grouped response ranges, and automatic documentation of previously registered routes remain deferred. Macros should eventually remove repetition from these ordinary Rust contracts rather than replace them with hidden behavior.

## Verification status

Test sources cover explicit resource fields, collection metadata, OpenAPI serialization, reusable schemas, request bodies, security schemes, template matching, duplicate rejection, unresolved references, and synchronized runtime/document registration. Compilation and runtime execution remain pending because Rust tooling is unavailable.
