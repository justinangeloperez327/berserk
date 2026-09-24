use berserk::{
    App, Headers, HealthCheck, HealthRegistry, HealthStatus, MemoryLogSink, Method, Metrics,
    MetricsLayer, Request, RequestId, RequestLogger, Response, TraceLayer,
};
use std::sync::Arc;

fn request(path: &str) -> Request {
    Request::new(
        Method::new("GET").unwrap(),
        path,
        Headers::new(),
        Vec::new(),
    )
    .unwrap()
}

#[test]
fn correlation_route_template_status_and_error_metrics_share_one_request_view() {
    let sink = Arc::new(MemoryLogSink::default());
    let metrics = Arc::new(Metrics::default());
    let mut app = App::new();
    app.middleware(RequestId);
    app.middleware(TraceLayer);
    app.middleware(RequestLogger::new(sink.clone()));
    app.middleware(MetricsLayer::new(metrics.clone()));
    app.route()
        .get("/users/{id}", |_id: String| {
            Response::text("down").status(503)
        })
        .unwrap();

    let response = app.handle(request("/users/secret?token=private")).unwrap();
    assert_eq!(response.status_code(), 503);
    assert!(response.headers().get("traceparent").is_some());

    let event = &sink.events()[0];
    assert_eq!(
        event.fields.get("path").map(String::as_str),
        Some("/users/{id}")
    );
    assert_eq!(event.fields.get("status").map(String::as_str), Some("503"));
    assert!(event.fields.contains_key("request_id"));
    assert!(event.fields.contains_key("trace_id"));
    let encoded = format!("{:?}", event);
    assert!(!encoded.contains("secret"));
    assert!(!encoded.contains("private"));

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.requests, 1);
    assert_eq!(snapshot.active, 0);
    assert_eq!(snapshot.failures, 1);
    assert_eq!(snapshot.server_errors, 1);
    assert_eq!(snapshot.client_errors, 0);
}

#[test]
fn client_and_server_errors_are_separate_fixed_cardinality_metrics() {
    let metrics = Arc::new(Metrics::default());
    let mut app = App::new();
    app.middleware(MetricsLayer::new(metrics.clone()));
    app.route()
        .get("/forbidden", || Response::text("no").status(403))
        .unwrap();
    app.route()
        .get("/failed", || Response::text("down").status(500))
        .unwrap();

    app.handle(request("/forbidden")).unwrap();
    app.handle(request("/failed")).unwrap();

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.client_errors, 1);
    assert_eq!(snapshot.server_errors, 1);
    assert_eq!(snapshot.failures, 1);
    let export = metrics.prometheus();
    assert!(export.contains("framework_http_client_errors_total 1"));
    assert!(export.contains("framework_http_server_errors_total 1"));
}

struct Dependency(bool);
impl HealthCheck for Dependency {
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
fn liveness_is_independent_while_readiness_reflects_dependency_health() {
    let mut health = HealthRegistry::default();
    health.add(Dependency(false)).unwrap();
    assert_eq!(health.liveness().unwrap().status_code(), 200);
    assert_eq!(health.readiness().unwrap().status_code(), 503);
    assert!(!health.snapshot().ready);
}
