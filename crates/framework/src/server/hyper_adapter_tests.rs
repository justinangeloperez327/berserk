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

fn empty_request() -> http::request::Builder {
    http::Request::builder().method("GET").uri("/")
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
        let config = ServerConfig {
            max_body_bytes: 4,
            ..ServerConfig::default()
        };
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

        let mut old_version = empty_request()
            .header("host", "example.test")
            .body(Full::new(Bytes::new()))
            .unwrap();
        *old_version.version_mut() = http::Version::HTTP_10;
        assert!(matches!(
            hyper_adapter::into_berserk_request(old_version, &ServerConfig::default()).await,
            Err(ProtocolError::UnsupportedVersion)
        ));
    });
}

#[test]
fn framing_host_and_header_limits_remain_strict() {
    runtime().block_on(async {
        for host in ["localhost", "example.com:80", "[::1]", "[::1]:3000"] {
            let request = empty_request()
                .header("host", host)
                .body(Full::new(Bytes::new()))
                .unwrap();
            assert!(
                hyper_adapter::into_berserk_request(request, &ServerConfig::default())
                    .await
                    .is_ok()
            );
        }

        for host in ["a b", "a:bad", "a:65536", "a@b", "[oops]", "::1", "a/b"] {
            let request = empty_request()
                .header("host", host)
                .body(Full::new(Bytes::new()))
                .unwrap();
            assert!(
                hyper_adapter::into_berserk_request(request, &ServerConfig::default())
                    .await
                    .is_err()
            );
        }

        let duplicate_length = http::Request::builder()
            .method("POST")
            .uri("/")
            .header("host", "example.test")
            .header("content-length", "1")
            .header("content-length", "1")
            .body(Full::new(Bytes::from_static(b"a")))
            .unwrap();
        assert!(matches!(
            hyper_adapter::into_berserk_request(duplicate_length, &ServerConfig::default()).await,
            Err(ProtocolError::Malformed)
        ));

        let conflicting_framing = http::Request::builder()
            .method("POST")
            .uri("/")
            .header("host", "example.test")
            .header("content-length", "1")
            .header("transfer-encoding", "chunked")
            .body(Full::new(Bytes::from_static(b"a")))
            .unwrap();
        assert!(matches!(
            hyper_adapter::into_berserk_request(conflicting_framing, &ServerConfig::default())
                .await,
            Err(ProtocolError::Malformed)
        ));

        let unsupported_encoding = http::Request::builder()
            .method("POST")
            .uri("/")
            .header("host", "example.test")
            .header("transfer-encoding", "gzip")
            .body(Full::new(Bytes::new()))
            .unwrap();
        assert!(matches!(
            hyper_adapter::into_berserk_request(unsupported_encoding, &ServerConfig::default())
                .await,
            Err(ProtocolError::UnsupportedTransferEncoding)
        ));

        let too_many_headers = empty_request()
            .header("host", "example.test")
            .header("x-extra", "1")
            .body(Full::new(Bytes::new()))
            .unwrap();
        let config = ServerConfig {
            max_headers: 1,
            ..ServerConfig::default()
        };
        assert!(matches!(
            hyper_adapter::into_berserk_request(too_many_headers, &config).await,
            Err(ProtocolError::HeaderLimit)
        ));
    });
}

#[test]
fn converts_buffered_response_with_content_length() {
    runtime().block_on(async {
        let response = hyper_adapter::into_wire_response(Response::text("hé"), false).unwrap();
        assert_eq!(response.status(), http::StatusCode::OK);
        assert_eq!(response.headers()[http::header::CONTENT_LENGTH], "3");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "hé");
    });
}

#[test]
fn response_lengths_head_and_bodyless_statuses_are_preserved() {
    runtime().block_on(async {
        for status in [204, 304] {
            let response =
                hyper_adapter::into_wire_response(Response::empty().status(status), false).unwrap();
            assert!(response
                .headers()
                .get(http::header::CONTENT_LENGTH)
                .is_none());
        }

        let reset =
            hyper_adapter::into_wire_response(Response::empty().status(205), false).unwrap();
        assert_eq!(reset.headers()[http::header::CONTENT_LENGTH], "0");

        let head = hyper_adapter::into_wire_response(Response::text("hello"), true).unwrap();
        assert_eq!(head.headers()[http::header::CONTENT_LENGTH], "5");
        assert!(head
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty());

        assert!(
            hyper_adapter::into_wire_response(Response::text("bad").status(204), false).is_err()
        );
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
