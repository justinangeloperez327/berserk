//! Test clients, assertions, recorders, and fakes for framework applications.
#![forbid(unsafe_code)]

mod event;
mod http;
mod job;
mod outbound;
mod temporary;

pub use event::EventRecorder;
pub use framework_notifications::MemoryMailTransport;
pub use http::{TestClient, TestRequest, TestResponse};
pub use job::{JobProbe, RecordingJob};
pub use outbound::FakeHttpClient;
pub use temporary::TemporaryDirectory;
