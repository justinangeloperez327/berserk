//! Test clients, assertions, recorders, and fakes for framework applications.
#![forbid(unsafe_code)]

mod event;
mod http;
mod job;
mod outbound;
mod temporary;

pub use berserk_notifications::MemoryMailTransport;
pub use event::EventRecorder;
/// HTTP test builders include `TestRequest::bearer` and 401/403 response assertions.
pub use http::{TestClient, TestRequest, TestResponse};
pub use job::{JobProbe, RecordingJob};
pub use outbound::FakeHttpClient;
pub use temporary::TemporaryDirectory;
