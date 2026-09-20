use berserk::{
    HealthCheck, HealthRegistry, HealthStatus, LogEvent, LogSink, MemoryLogSink, MetricsSnapshot,
    TraceContext,
};
use std::collections::BTreeMap;

struct DatabaseHealth(HealthStatus);

impl HealthCheck for DatabaseHealth {
    fn name(&self) -> &'static str {
        "database"
    }

    fn check(&self) -> HealthStatus {
        self.0
    }
}

#[test]
fn health_snapshot_is_available_without_rendering_http() {
    let mut registry = HealthRegistry::default();
    registry.add(DatabaseHealth(HealthStatus::Healthy)).unwrap();

    let snapshot = registry.snapshot();
    assert!(snapshot.ready);
    assert_eq!(snapshot.checks.len(), 1);
    assert_eq!(snapshot.checks[0].name, "database");
    assert_eq!(snapshot.checks[0].status, HealthStatus::Healthy);
    assert_eq!(registry.readiness().unwrap().status_code(), 200);

    let mut unavailable = HealthRegistry::default();
    unavailable
        .add(DatabaseHealth(HealthStatus::Unhealthy))
        .unwrap();
    assert!(!unavailable.snapshot().ready);
    assert_eq!(unavailable.readiness().unwrap().status_code(), 503);
}

#[test]
fn metrics_snapshot_exposes_operational_summaries() {
    let snapshot = MetricsSnapshot {
        requests: 12,
        active: 2,
        failures: 1,
        rate_limited: 3,
        duration_us: 1_000,
    };

    assert_eq!(snapshot.completed(), 10);
    assert_eq!(snapshot.average_duration_us(), Some(100));
    assert!(!snapshot.healthy());

    let empty = MetricsSnapshot::default();
    assert_eq!(empty.average_duration_us(), None);
    assert!(empty.healthy());
}

#[test]
fn memory_logs_can_be_inspected_drained_and_cleared() {
    let sink = MemoryLogSink::default();
    sink.emit(LogEvent {
        name: "test.event",
        timestamp_ms: 1,
        fields: BTreeMap::from([("key".into(), "value".into())]),
    });

    assert_eq!(sink.events().len(), 1);
    assert_eq!(sink.take().len(), 1);
    assert!(sink.events().is_empty());

    sink.emit(LogEvent {
        name: "test.event",
        timestamp_ms: 2,
        fields: BTreeMap::new(),
    });
    sink.clear();
    assert!(sink.events().is_empty());
}

#[test]
fn trace_context_exposes_flags_and_sampling_state() {
    let sampled =
        TraceContext::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01").unwrap();
    assert_eq!(sampled.flags(), "01");
    assert!(sampled.sampled());

    let unsampled =
        TraceContext::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00").unwrap();
    assert_eq!(unsampled.flags(), "00");
    assert!(!unsampled.sampled());
}
