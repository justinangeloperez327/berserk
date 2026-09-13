# Phase 19 — Authentication and authorization

Phase 19 adds an optional `auth` crate and HTTP integration. It supplies secure primitives and contracts while leaving application-specific user storage and policy decisions with the application.

Enable the main integration with the `auth` feature:

```toml
framework = { path = "../../crates/framework", features = ["auth"] }
```

## Password authentication

```rust
let passwords = Argon2Passwords;
let encoded = passwords.hash(&Secret::new("user password"))?;

let authenticator = PasswordAuthenticator::new(user_provider, passwords)?;
let principal = authenticator.authenticate(email, &Secret::new(password))?;
```

`IdentityProvider` is the application boundary for loading a password hash and principal. `PasswordAuthenticator` returns the same invalid-credentials error for unknown accounts, disabled accounts, and incorrect passwords. Unknown identifiers are checked against a generated dummy Argon2 hash to reduce username timing disclosure.

Passwords use the vetted RustCrypto Argon2 implementation and PHC-formatted hashes. Every hash receives a fresh operating-system-generated salt. `Secret` redacts debug output and zeroizes its owned string when dropped. Password complexity, breach checks, reset links, email verification, MFA, and rate limiting remain application or later-phase responsibilities.

## Sessions and bearer tokens

```rust
let sessions = SessionManager::new(store, Duration::from_secs(3600))?;
let token = sessions.issue(principal, now)?;

sessions.revoke(&token)?;
sessions.prune(now)?;
```

Session tokens contain 256 random bits from the operating system and are exposed once to the application. Token debug output is redacted and token memory is zeroized on drop. `SessionStore` receives only a SHA-256 token digest, the principal, and explicit issue/expiry timestamps; raw bearer tokens are never stored by the supplied implementation.

`MemorySessionStore` supports development and testing, not durable production sessions. Applications can implement `SessionStore` using the database or another backend. Expired and revoked sessions fail authentication. Session rotation can be implemented as issuing a replacement and revoking the old token.

## HTTP guard

```rust
app.middleware(Authenticated::new(session_manager));

app.get("/account", |request: Request| {
    Response::text(request.principal().unwrap().subject())
})?;
```

`Authenticated<G>` accepts exactly one `Authorization: Bearer <token>` credential. A missing, malformed, expired, or rejected credential returns `401` with `WWW-Authenticate: Bearer`. Successful authentication attaches a typed `Principal` to the request. Store or cryptographic failures remain internal framework errors rather than being presented as bad credentials.

Cookie sessions, CSRF protection, OAuth/OIDC redirects, JWT validation, API-key formats, refresh tokens, and multi-guard selection are not silently enabled. They require explicit later contracts because their threat models differ.

## Gates and policies

```rust
let edit = Ability::new("posts.update")?;
gate.define(edit.clone(), |principal| {
    if principal.has_role("editor") { Decision::Allow } else { Decision::Deny }
})?;
gate.authorize(&principal, &edit)?;
```

Gates cover broad abilities. `Policy<Resource>` provides resource-aware decisions. Ability names are validated, duplicate definitions fail during setup, and missing abilities deny access. Public forbidden errors do not include internal policy details.

## Verification status

Test sources cover Argon2 hash/verify behavior, secret redaction, generic credential failures, token expiration and revocation, role gates, undefined and duplicate abilities, bearer challenges, and principal attachment. Compilation and runtime execution remain pending because Rust tooling is unavailable in this environment.
