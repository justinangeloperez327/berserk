use berserk::{
    App, Headers, HealthCheck, HealthRegistry, HealthStatus, MemoryLogSink, Method, Metrics,
    MetricsLayer, RateLimitLayer, RateLimiter, Request, RequestId, RequestLogger, Response,
    TraceContext, TraceLayer,
};
use std::{sync::Arc, time::Duration};

fn request(target: &str) -> Request {
    Request::new(Method::new("GET").unwrap(), target, Headers::new(), vec![]).unwrap()
}

#[test]
fn request_logs_are_structured_and_exclude_query_values() {
    let sink = Arc::new(MemoryLogSink::default());
    let mut app = App::new();
    app.middleware(RequestId);
    app.middleware(RequestLogger::new(sink.clone()));
    app.route().get("/users", || Response::text("OK")).unwrap();
    app.handle(request("/users?token=secret")).unwrap();

    let events = sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].name, "http.request");
    assert_eq!(
        events[0].fields.get("path").map(String::as_str),
        Some("/users")
    );
    assert!(!format!("{:?}", events[0]).contains("secret"));
    assert!(events[0].fields.contains_key("request_id"));
}

#[test]
fn metrics_track_requests_failures_active_work_and_fixed_export_names() {
    let metrics = Arc::new(Metrics::default());
    let mut app = App::new();
    app.middleware(MetricsLayer::new(metrics.clone()));
    app.route()
        .get("/failed", || Response::text("down").status(503))
        .unwrap();
    app.handle(request("/failed")).unwrap();
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.requests, 1);
    assert_eq!(snapshot.failures, 1);
    assert_eq!(snapshot.active, 0);
    assert!(metrics
        .prometheus()
        .contains("framework_http_requests_total 1"));
}

struct DatabaseHealth(bool);
impl HealthCheck for DatabaseHealth {
    fn name(&self) -> &'static str {
        "database"
    }
    fn check(&self) -> HealthStatus {
        if self.0 {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        }
    }
}

#[test]
fn readiness_reports_named_checks_without_internal_diagnostics() {
    let mut health = HealthRegistry::default();
    health.add(DatabaseHealth(false)).unwrap();
    let response = health.readiness().unwrap();
    assert_eq!(response.status_code(), 503);
    let body = std::str::from_utf8(response.body()).unwrap();
    assert!(body.contains("database"));
    assert!(body.contains("not_ready"));
    assert_eq!(health.liveness().unwrap().status_code(), 200);
}

#[test]
fn trace_context_validates_parent_headers_and_creates_child_spans() {
    let parent =
        TraceContext::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01").unwrap();
    let child = parent.child();
    assert_eq!(child.trace_id(), parent.trace_id());
    assert_ne!(child.parent_id(), parent.parent_id());
    assert!(
        TraceContext::parse("00-00000000000000000000000000000000-00f067aa0ba902b7-01").is_none()
    );

    let mut app = App::new();
    app.middleware(TraceLayer);
    app.route()
        .get("/trace", |request: Request| {
            Response::text(request.trace_context().unwrap().trace_id())
        })
        .unwrap();
    let response = app.handle(request("/trace")).unwrap();
    assert!(response.headers().get("traceparent").is_some());
}

#[test]
fn rate_limits_are_bounded_and_return_retry_metadata() {
    let limiter = Arc::new(RateLimiter::new(1, Duration::from_secs(60), 2).unwrap());
    assert!(limiter.check("one", 100).unwrap().allowed);
    assert!(!limiter.check("one", 101).unwrap().allowed);
    assert!(limiter.check("two", 101).unwrap().allowed);
    assert!(!limiter.check("three", 101).unwrap().allowed);

    let mut app = App::new();
    app.middleware(RateLimitLayer::global(Arc::new(
        RateLimiter::new(1, Duration::from_secs(60), 1).unwrap(),
    )));
    app.route()
        .get("/limited", || Response::text("OK"))
        .unwrap();
    assert_eq!(app.handle(request("/limited")).unwrap().status_code(), 200);
    let rejected = app.handle(request("/limited")).unwrap();
    assert_eq!(rejected.status_code(), 429);
    assert!(rejected.headers().get("retry-after").is_some());
}
