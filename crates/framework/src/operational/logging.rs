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
        let path = request.path().to_owned();
        let request_id = request.request_id().map(str::to_owned);
        let trace_id = request
            .trace_context()
            .map(|context| context.trace_id().to_owned());
        let result = next.run(request);
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
            Err(_) => {
                fields.insert("status".into(), "500".into());
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
