use berserk::{App, Json, Response};
use berserk_client::{
    Header, HttpClient, Method as ClientMethod, Request as ClientRequest,
    Response as ClientResponse, Url,
};
use berserk_events::{Event, EventBus};
use berserk_jobs::{MemoryFailedJobs, QueueConfig, RetryPolicy, WorkerPool};
use berserk_testing::{
    EventRecorder, FakeHttpClient, RecordingJob, TemporaryDirectory, TestClient,
};
use std::{collections::BTreeMap, sync::Arc};

#[test]
fn in_memory_requests_support_fluent_assertions() {
    let mut app = App::new();
    app.get("/hello", || Response::text("hello").header("x-test", "yes"))
        .unwrap();
    TestClient::new(&app)
        .get("/hello")
        .unwrap()
        .assert_ok()
        .assert_status(200)
        .assert_header("x-test", "yes")
        .assert_text("hello");
}

#[test]
fn json_assertions_compare_structure() {
    let expected = Json::Object(BTreeMap::from([("ok".into(), Json::Bool(true))]));
    let response_value = expected.clone();
    let mut app = App::new();
    app.get("/json", move || Response::json(&response_value))
        .unwrap();
    TestClient::new(&app)
        .get("/json")
        .unwrap()
        .assert_json(&expected);
}

#[test]
fn fake_http_records_requests_and_uses_queued_responses() {
    let fake = FakeHttpClient::new();
    fake.push_response(
        ClientResponse::new(202, vec![Header::new("x-fake", "yes").unwrap()], Vec::new()).unwrap(),
    );
    let request = ClientRequest::new(
        ClientMethod::Post,
        Url::parse("https://example.com/hook").unwrap(),
    )
    .body(b"payload".to_vec());
    assert_eq!(fake.send(request).unwrap().status(), 202);
    assert_eq!(fake.requests()[0].body_bytes(), b"payload");
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Created(u64);
impl Event for Created {
    const NAME: &'static str = "created";
}

#[test]
fn event_and_job_recorders_capture_behavior() {
    let bus = EventBus::new();
    let recorder = EventRecorder::<Created>::new();
    bus.listen::<Created, _>(recorder.listener()).unwrap();
    bus.dispatch(&Created(7)).unwrap();
    assert_eq!(recorder.events(), vec![Created(7)]);
    let failures = Arc::new(MemoryFailedJobs::new(4).unwrap());
    let pool = WorkerPool::new(
        QueueConfig {
            workers: 1,
            capacity: 2,
        },
        failures,
    )
    .unwrap();
    let (job, probe) = RecordingJob::failing("recording", 1);
    pool.queue()
        .dispatch(
            job,
            RetryPolicy {
                max_attempts: 2,
                ..RetryPolicy::none()
            },
        )
        .unwrap();
    pool.shutdown().unwrap();
    assert_eq!(probe.attempts(), vec![1, 2]);
}

#[test]
fn temporary_directories_clean_up_their_contents() {
    let path = {
        let directory = TemporaryDirectory::new().unwrap();
        std::fs::write(directory.path().join("file"), b"test").unwrap();
        directory.path().to_path_buf()
    };
    assert!(!path.exists());
}
