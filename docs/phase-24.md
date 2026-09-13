# Phase 24 — External Communication

Phase 24 adds optional outbound HTTP and notification components while keeping network security choices explicit.

## HTTP client

`framework-client` defines `HttpClient`, validated `Url`, `Header`, `Request`, and `Response` types, and structured client errors. External TLS-capable clients can implement the same synchronous trait.

`TcpHttpClient` is a bounded standard-library transport for plaintext `http://` connections. It owns Host, Connection, Content-Length, and Transfer-Encoding framing; validates request header/body limits; uses connect/read/write timeouts; and bounds response headers, header count, body bytes, chunks, and trailers. It accepts content-length, chunked, and connection-close response bodies. It returns redirects rather than following them and does not manage cookies.

The built-in transport rejects `https://`. It never downgrades secure URLs. Real external APIs should use a vetted TLS adapter with certificate and hostname verification. Plaintext transport is appropriate for controlled local development or trusted internal networks only.

Debug implementations omit HTTP header values and body contents. URL debug output redacts the path and query because webhook URLs and query parameters often contain credentials. Applications must still control server-side request forgery by validating destinations or using an allowlist; this framework cannot infer which hosts an application is authorized to contact.

## Mail and notifications

`framework-notifications` provides safe email/message types, `MailTransport`, a bounded `MemoryMailTransport` test fake, webhook messages, recipients, and a Laravel-inspired `Notification` trait. A notification can produce mail, webhook, both, or neither for a recipient.

`Notifier` applies configurable body limits and returns one `DeliveryOutcome` per produced channel. A missing transport is an explicit failed outcome. Webhooks require HTTPS by default; local plaintext use requires the visible `allow_insecure_webhooks` opt-in.

Retries are synchronous and use fixed backoff. Network and transport failures, HTTP `408`, `429`, and `5xx` are retryable. Ordinary `4xx` responses are terminal. Retrying may duplicate a side effect when the remote system completed it but the response was lost, so webhook messages can carry an idempotency key and receiving services should enforce it.

Authenticated SMTP, MIME attachments, DKIM, TLS, proxy configuration, connection pooling, cookie jars, and provider SDKs are not implemented by the standard-library components. They belong in vetted optional adapters. Secret credentials must live in validated application configuration and must never be placed in debug output or error bodies.

## Feature selection

```toml
framework = { path = "crates/framework", features = ["client", "notifications"] }
```

Enabling `notifications` automatically enables `client`. Applications may depend on either component crate directly.

## Verification status

Contract tests cover URL/header validation, plaintext loopback requests, content-length and chunked decoding, bounds, HTTPS rejection, email/webhook construction, channel reporting, retry classification, missing transports, and injection rejection. Rust tooling is unavailable in the preparation environment, so compilation and test execution remain pending.
