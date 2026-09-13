# Phase 21 — Operational features

Phase 21 adds production-facing diagnostics and controls without changing the synchronous server model.

## Structured request logging

```rust
app.middleware(RequestId);
app.middleware(TraceLayer);
app.middleware(RequestLogger::new(StderrJson));
```

`RequestLogger` emits one `http.request` event after each request with method, path, status, duration, request ID, and trace ID when available. It records the matched request path without query values and never records headers or bodies. Sink failures do not fail successful HTTP requests. `StderrJson` emits one JSON object per line; `MemoryLogSink` supports tests.

Applications must still avoid putting secrets or user identifiers in route paths. Authentication tokens, cookies, query strings, and bodies are intentionally absent from the built-in event.

## Metrics

```rust
let metrics = Arc::new(Metrics::default());
app.middleware(MetricsLayer::new(metrics.clone()));

app.get("/metrics", move |_| {
    metrics.response()
})?;
```

The built-in metrics use fixed names and no user-controlled labels: total requests, active requests, failures, rate-limited requests, and cumulative request duration. This prevents unbounded metric cardinality. `MetricsSnapshot` supports programmatic inspection and `prometheus()` provides a text exposition.

Middleware order is explicit. Put `MetricsLayer` outside other layers to count their early responses. Give `RateLimitLayer` the same metrics handle when rate-limit rejection counts are required regardless of ordering.

## Health checks

```rust
health.add(DatabaseHealthCheck)?;
app.get("/live", move |_| health.liveness())?;
app.get("/ready", move |_| health.readiness())?;
```

Liveness only reports that the process can serve the handler. Readiness runs up to 64 uniquely named checks and returns `200` when all are healthy or `503` otherwise. Public JSON contains check names and healthy/unhealthy states, not internal error details.

Checks run synchronously, so they must be fast and bounded. Expensive network checks should use a separately refreshed cached state rather than blocking every readiness request.

## Trace context

`TraceLayer` accepts valid W3C `traceparent` version `00`, preserves its trace ID, creates a new random parent/span ID, attaches a typed `TraceContext` to the request, and returns the current trace header. Missing or malformed context starts a new trace using operating-system randomness. All-zero IDs are rejected.

This is propagation infrastructure, not a complete exporter. OpenTelemetry exporters and span backends can integrate later through the typed context and structured log sink.

## Rate limiting

```rust
let limiter = Arc::new(RateLimiter::new(100, Duration::from_secs(60), 10_000)?);
app.middleware(RateLimitLayer::keyed(limiter, |request| {
    application_defined_client_key(request)
}));
```

The limiter uses fixed windows, explicit request limits, and a strict maximum number of stored keys. Keys are limited to 128 bytes. Expired windows are pruned before admitting new keys. When capacity is exhausted, unknown keys fail closed instead of growing memory. Rejections return `429`, `Retry-After`, and rate-limit metadata.

Client identity is intentionally supplied by the application. Do not trust `X-Forwarded-For` unless a trusted reverse proxy strips incoming values and writes the authoritative client address. Global limiting is available when per-client identity is unnecessary.

The in-memory limiter is process-local. Distributed deployments need a shared atomic backend in a later adapter; multiplying a per-process limit by the number of instances is otherwise expected.

## Deferred scope

OpenTelemetry exporters, histogram buckets, route-template labels, log filtering/redaction configuration, distributed rate-limit stores, proxy trust configuration, circuit breakers, cached health polling, alert rules, and operational dashboards remain deferred.

## Verification status

Test sources cover structured log fields and query exclusion, request/failure/active metrics, Prometheus names, readiness outcomes, trace validation and child spans, bounded key capacity, fixed-window decisions, and HTTP `429` metadata. Compilation and runtime execution remain pending because Rust tooling is unavailable.
