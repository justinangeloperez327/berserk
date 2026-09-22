use crate::{Json, Middleware, Next, Request, Response, Result};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogEvent {
    pub name: &'static str,
    pub timestamp_ms: u128,
    pub fields: BTreeMap<String, String>,
}

pub trait LogSink: Send + Sync + 'static {
    fn emit(&self, event: LogEvent);
}
impl<T: LogSink + ?Sized> LogSink for Arc<T> {
    fn emit(&self, event: LogEvent) {
        (**self).emit(event);
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StderrJson;
impl LogSink for StderrJson {
    fn emit(&self, event: LogEvent) {
        let mut object = BTreeMap::from([
            ("event".into(), Json::from(event.name)),
            ("timestamp_ms".into(), Json::from(event.timestamp_ms)),
        ]);
        for (key, value) in event.fields {
            object.insert(key, Json::from(value));
        }
        if let Ok(line) = Json::Object(object).encode() {
            eprintln!("{line}");
        }
    }
}

#[derive(Default)]
pub struct MemoryLogSink(Mutex<Vec<LogEvent>>);
impl MemoryLogSink {
    pub fn events(&self) -> Vec<LogEvent> {
        self.0
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }

    pub fn take(&self) -> Vec<LogEvent> {
        self.0
            .lock()
            .map(|mut events| std::mem::take(&mut *events))
            .unwrap_or_default()
    }

    pub fn clear(&self) {
        if let Ok(mut events) = self.0.lock() {
            events.clear();
        }
    }
}
impl LogSink for MemoryLogSink {
    fn emit(&self, event: LogEvent) {
        if let Ok(mut events) = self.0.lock() {
            events.push(event);
        }
    }
}

pub struct RequestLogger<S> {
    sink: S,
}
impl<S> RequestLogger<S> {
    pub fn new(sink: S) -> Self {
        Self { sink }
    }
}

impl<S: LogSink> Middleware for RequestLogger<S> {
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {
        let start = Instant::now();
        let method = request.method().as_str().to_owned();
        let route_pattern = request.route_pattern_context();
        let request_id = request.request_id().map(str::to_owned);
        let trace_id = request
            .trace_context()
            .map(|context| context.trace_id().to_owned());
        let result = next.run(request);
        let path = route_pattern
            .get()
            .map(String::as_str)
            .unwrap_or("<unmatched>")
            .to_owned();
        let mut fields = BTreeMap::from([
            ("method".into(), method),
            ("path".into(), path),
            (
                "duration_us".into(),
                start.elapsed().as_micros().to_string(),
            ),
        ]);
        if let Some(value) = request_id {
            fields.insert("request_id".into(), value);
        }
        if let Some(value) = trace_id {
            fields.insert("trace_id".into(), value);
        }
        match &result {
            Ok(response) => {
                fields.insert("status".into(), response.status_code().to_string());
            }
            Err(error) => {
                fields.insert("status".into(), error.status_code().to_string());
                fields.insert("outcome".into(), "error".into());
            }
        }
        self.sink.emit(LogEvent {
            name: "http.request",
            timestamp_ms: now_ms(),
            fields,
        });
        result
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{App, Headers, Method};
    use std::sync::Arc;

    fn request(path: &str) -> Request {
        Request::new(Method::new("GET").unwrap(), path, Headers::new(), vec![]).unwrap()
    }

    #[test]
    fn request_logger_uses_route_template_without_parameter_or_query_values() {
        let sink = Arc::new(MemoryLogSink::default());
        let mut app = App::new();
        app.middleware(RequestLogger::new(sink.clone()));
        app.route()
            .get("/users/{id}", |_id: String| Response::text("ok"))
            .unwrap();

        app.handle(request("/users/secret-account?token=private-value"))
            .unwrap();

        let events = sink.events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].fields.get("path").map(String::as_str),
            Some("/users/{id}")
        );
        let encoded = format!("{:?}", events[0].fields);
        assert!(!encoded.contains("secret-account"));
        assert!(!encoded.contains("private-value"));
    }

    #[test]
    fn request_logger_does_not_log_unmatched_request_targets() {
        let sink = Arc::new(MemoryLogSink::default());
        let mut app = App::new();
        app.middleware(RequestLogger::new(sink.clone()));

        let response = app
            .handle(request("/missing/secret-value?token=private-value"))
            .unwrap();
        assert_eq!(response.status_code(), 404);

        let events = sink.events();
        assert_eq!(
            events[0].fields.get("path").map(String::as_str),
            Some("<unmatched>")
        );
        let encoded = format!("{:?}", events[0].fields);
        assert!(!encoded.contains("secret-value"));
        assert!(!encoded.contains("private-value"));
    }

    #[test]
    fn request_logger_preserves_client_error_status() {
        let sink = Arc::new(MemoryLogSink::default());
        let mut app = App::new();
        app.middleware(RequestLogger::new(sink.clone()));
        app.route()
            .get("/forbidden", || -> Result<Response> {
                Err(crate::Error::forbidden())
            })
            .unwrap();

        let error = app.handle(request("/forbidden")).unwrap_err();
        assert_eq!(error.status_code(), 403);

        let events = sink.events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].fields.get("status").map(String::as_str),
            Some("403")
        );
        assert_eq!(
            events[0].fields.get("outcome").map(String::as_str),
            Some("error")
        );
    }
}
