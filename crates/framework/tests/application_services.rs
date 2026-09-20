#![cfg(all(
    feature = "cache",
    feature = "storage",
    feature = "events",
    feature = "jobs",
    feature = "client",
    feature = "notifications"
))]

use berserk::{App, Headers, Method, Request, Response};
use berserk::cache::MemoryCache;
use berserk::client::{HttpClient, Request as ClientRequest, Response as ClientResponse};
use berserk::events::EventBus;
use berserk::jobs::{MemoryFailedJobs, QueueConfig, WorkerPool};
use berserk::notifications::{Notifier, NotifierConfig};
use berserk::storage::MemoryStorage;
use std::sync::Arc;

struct StaticClient;

impl HttpClient for StaticClient {
    fn send(&self, _request: ClientRequest) -> berserk::client::Result<ClientResponse> {
        ClientResponse::new(204, Vec::new(), Vec::new())
    }
}

fn request() -> Request {
    Request::new(Method::new("GET").unwrap(), "/", Headers::new(), Vec::new()).unwrap()
}

#[test]
fn application_services_are_registered_once_and_available_from_requests() {
    let failures = Arc::new(MemoryFailedJobs::new(8).unwrap());
    let pool = WorkerPool::new(
        QueueConfig {
            workers: 1,
            capacity: 8,
        },
        failures,
    )
    .unwrap();

    let mut app = App::new();
    app.cache(MemoryCache::new(8, 1024).unwrap()).unwrap();
    app.storage(MemoryStorage::new(8, 1024).unwrap()).unwrap();
    app.events(EventBus::new()).unwrap();
    app.jobs(pool.queue()).unwrap();
    app.http_client(StaticClient).unwrap();
    app.notifications(Notifier::new(NotifierConfig::default()).unwrap())
        .unwrap();

    assert!(app.cache(MemoryCache::new(1, 1).unwrap()).is_err());

    app.route()
        .get("/", |request: Request| -> berserk::Result<Response> {
            let cache = request.cache()?;
            let storage = request.storage()?;
            let events = request.events()?;
            let jobs = request.jobs()?;
            let client = request.http_client()?;
            let notifications = request.notifications()?;

            assert!(Arc::strong_count(&cache) >= 2);
            assert!(Arc::strong_count(&storage) >= 2);
            assert!(Arc::strong_count(&events) >= 2);
            assert!(!jobs.snapshot()?.closed);
            assert!(Arc::strong_count(&client) >= 2);
            assert!(Arc::strong_count(&notifications) >= 2);

            Ok(Response::text("ok"))
        })
        .unwrap();

    assert_eq!(app.respond(request()).status_code(), 200);
    pool.shutdown().unwrap();
}
