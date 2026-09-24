use crate::{Middleware, Next, Request, Response, Result};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;

#[derive(Debug, Default)]
pub struct Metrics {
    requests: AtomicU64,
    active: AtomicU64,
    failures: AtomicU64,
    rate_limited: AtomicU64,
    duration_us: AtomicU64,
    client_errors: AtomicU64,
    server_errors: AtomicU64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MetricsSnapshot {
    pub requests: u64,
    pub active: u64,
    pub failures: u64,
    pub rate_limited: u64,
    pub duration_us: u64,
    pub client_errors: u64,
    pub server_errors: u64,
}

impl MetricsSnapshot {
    pub fn completed(&self) -> u64 {
        self.requests.saturating_sub(self.active)
    }

    pub fn average_duration_us(&self) -> Option<u64> {
        let completed = self.completed();
        (completed > 0).then(|| self.duration_us / completed)
    }

    pub fn healthy(&self) -> bool {
        self.failures == 0
    }
}

impl Metrics {
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            requests: self.requests.load(Ordering::Relaxed),
            active: self.active.load(Ordering::Relaxed),
            failures: self.failures.load(Ordering::Relaxed),
            rate_limited: self.rate_limited.load(Ordering::Relaxed),
            duration_us: self.duration_us.load(Ordering::Relaxed),
            client_errors: self.client_errors.load(Ordering::Relaxed),
            server_errors: self.server_errors.load(Ordering::Relaxed),
        }
    }
    pub fn prometheus(&self) -> String {
        let value = self.snapshot();
        format!(concat!(
            "# TYPE framework_http_requests_total counter\nframework_http_requests_total {}\n",
            "# TYPE framework_http_active gauge\nframework_http_active {}\n",
            "# TYPE framework_http_failures_total counter\nframework_http_failures_total {}\n",
            "# TYPE framework_http_rate_limited_total counter\nframework_http_rate_limited_total {}\n",
            "# TYPE framework_http_duration_microseconds_total counter\nframework_http_duration_microseconds_total {}\n",
            "# TYPE framework_http_client_errors_total counter\nframework_http_client_errors_total {}\n",
            "# TYPE framework_http_server_errors_total counter\nframework_http_server_errors_total {}\n"
        ), value.requests, value.active, value.failures, value.rate_limited, value.duration_us, value.client_errors, value.server_errors)
    }
    pub fn response(&self) -> Result<Response> {
        Response::text(self.prometheus()).header("content-type", "text/plain; version=0.0.4")
    }
    pub(crate) fn mark_rate_limited(&self) {
        self.rate_limited.fetch_add(1, Ordering::Relaxed);
    }
}

pub struct MetricsLayer {
    metrics: Arc<Metrics>,
}
impl MetricsLayer {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        Self { metrics }
    }
    pub fn metrics(&self) -> &Arc<Metrics> {
        &self.metrics
    }
}
impl Middleware for MetricsLayer {
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {
        self.metrics.requests.fetch_add(1, Ordering::Relaxed);
        self.metrics.active.fetch_add(1, Ordering::Relaxed);
        let guard = ActiveGuard(&self.metrics.active);
        let start = Instant::now();
        let result = next.run(request);
        let elapsed = u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX);
        self.metrics
            .duration_us
            .fetch_add(elapsed, Ordering::Relaxed);
        let status = match &result {
            Ok(response) => response.status_code(),
            Err(error) => error.status_code(),
        };
        if (400..500).contains(&status) {
            self.metrics.client_errors.fetch_add(1, Ordering::Relaxed);
        }
        if status >= 500 {
            self.metrics.failures.fetch_add(1, Ordering::Relaxed);
            self.metrics.server_errors.fetch_add(1, Ordering::Relaxed);
        }
        drop(guard);
        result
    }
}

struct ActiveGuard<'a>(&'a AtomicU64);
impl Drop for ActiveGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{App, Headers, Method};

    fn request(path: &str) -> Request {
        Request::new(Method::new("GET").unwrap(), path, Headers::new(), vec![]).unwrap()
    }

    #[test]
    fn client_errors_are_not_counted_as_server_failures() {
        let metrics = Arc::new(Metrics::default());
        let mut app = App::new();
        app.middleware(MetricsLayer::new(metrics.clone()));
        app.route()
            .get("/forbidden", || -> Result<Response> {
                Err(crate::Error::forbidden())
            })
            .unwrap();

        let error = app.handle(request("/forbidden")).unwrap_err();
        assert_eq!(error.status_code(), 403);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.requests, 1);
        assert_eq!(snapshot.failures, 0);
        assert_eq!(snapshot.active, 0);
    }

    #[test]
    fn server_errors_are_counted_as_failures() {
        let metrics = Arc::new(Metrics::default());
        let mut app = App::new();
        app.middleware(MetricsLayer::new(metrics.clone()));
        app.route()
            .get("/failed", || -> Result<Response> {
                Err(crate::Error::rejected(500, "failed"))
            })
            .unwrap();

        let error = app.handle(request("/failed")).unwrap_err();
        assert_eq!(error.status_code(), 500);
        assert_eq!(metrics.snapshot().failures, 1);
    }
}
