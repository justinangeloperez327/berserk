use super::{hyper_adapter, ProtocolError};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use std::{
    io::{self, Read},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::{Response, ServerConfig};

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime")
}

#[test]
fn converts_bounded_http_request_to_berserk_request() {
    runtime().block_on(async {
        let request = http::Request::builder()
            .method("POST")
            .uri("/users?active=1")
            .header("host", "example.test")
            .header("content-length", "5")
            .body(Full::new(Bytes::from_static(b"hello")))
            .unwrap();

        let request = hyper_adapter::into_berserk_request(request, &ServerConfig::default())
            .await
            .unwrap();
        assert_eq!(request.method().as_str(), "POST");
        assert_eq!(request.path(), "/users");
        assert_eq!(request.query_string(), Some("active=1"));
        assert_eq!(request.body(), b"hello");
    });
}

#[test]
fn rejects_body_over_configured_limit() {
    runtime().block_on(async {
        let mut config = ServerConfig::default();
        config.max_body_bytes = 4;
        let request = http::Request::builder()
            .method("POST")
            .uri("/users")
            .header("host", "example.test")
            .header("transfer-encoding", "chunked")
            .body(Full::new(Bytes::from_static(b"hello")))
            .unwrap();

        let error = hyper_adapter::into_berserk_request(request, &config)
            .await
            .unwrap_err();
        assert!(matches!(error, ProtocolError::BodyLimit));
    });
}

#[test]
fn rejects_unsafe_or_unsupported_request_forms() {
    runtime().block_on(async {
        let absolute = http::Request::builder()
            .uri("http://example.test/users")
            .header("host", "example.test")
            .body(Full::new(Bytes::new()))
            .unwrap();
        assert!(matches!(
            hyper_adapter::into_berserk_request(absolute, &ServerConfig::default()).await,
            Err(ProtocolError::Malformed)
        ));

        let duplicate_host = http::Request::builder()
            .uri("/users")
            .header("host", "example.test")
            .header("host", "other.test")
            .body(Full::new(Bytes::new()))
            .unwrap();
        assert!(matches!(
            hyper_adapter::into_berserk_request(duplicate_host, &ServerConfig::default()).await,
            Err(ProtocolError::Malformed)
        ));

        let expectation = http::Request::builder()
            .uri("/users")
            .header("host", "example.test")
            .header("expect", "100-continue")
            .body(Full::new(Bytes::new()))
            .unwrap();
        assert!(matches!(
            hyper_adapter::into_berserk_request(expectation, &ServerConfig::default()).await,
            Err(ProtocolError::UnsupportedExpectation)
        ));
    });
}

#[test]
fn converts_buffered_response_with_content_length() {
    runtime().block_on(async {
        let response = hyper_adapter::into_wire_response(Response::text("hello"), false).unwrap();
        assert_eq!(response.status(), http::StatusCode::OK);
        assert_eq!(response.headers()[http::header::CONTENT_LENGTH], "5");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "hello");
    });
}

#[test]
fn streams_synchronous_reader_off_the_async_executor() {
    runtime().block_on(async {
        let response = hyper_adapter::into_wire_response(
            Response::stream(std::io::Cursor::new(b"streamed".to_vec())),
            false,
        )
        .unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "streamed");
    });
}

struct CountingReader {
    reads: Arc<AtomicUsize>,
}

impl Read for CountingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(0)
    }
}

#[test]
fn head_does_not_consume_stream_source() {
    runtime().block_on(async {
        let reads = Arc::new(AtomicUsize::new(0));
        let response = Response::stream(CountingReader {
            reads: Arc::clone(&reads),
        });
        let response = hyper_adapter::into_wire_response(response, true).unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.is_empty());
        assert_eq!(reads.load(Ordering::SeqCst), 0);
    });
}
