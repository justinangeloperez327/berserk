use framework_client::{
    ErrorKind, Header, HttpClient, Method, Request, TcpClientConfig, TcpHttpClient, Url,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

#[test]
fn urls_and_headers_validate_boundaries() {
    let url = Url::parse("http://127.0.0.1:8080/path?q=1").unwrap();
    assert_eq!(url.host(), "127.0.0.1");
    assert_eq!(url.port(), 8080);
    assert_eq!(url.path_and_query(), "/path?q=1");
    assert_eq!(
        Url::parse("ftp://example.com").unwrap_err().kind(),
        ErrorKind::UnsupportedScheme
    );
    assert!(Header::new("x-test", "bad\r\nvalue").is_err());
    assert!(!format!("{:?}", Header::new("authorization", "secret").unwrap()).contains("secret"));
}

#[test]
fn tcp_client_decodes_content_length() {
    let (url, server) =
        server_once(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nX-Test: yes\r\n\r\nhello");
    let response = TcpHttpClient::default()
        .send(Request::new(Method::Get, url))
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.header("x-test"), Some("yes"));
    assert_eq!(response.text().unwrap(), "hello");
    server.join().unwrap();
}

#[test]
fn tcp_client_decodes_chunked_bodies() {
    let (url, server) = server_once(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
    );
    let response = TcpHttpClient::default()
        .send(Request::new(Method::Get, url))
        .unwrap();
    assert_eq!(response.body(), b"hello");
    server.join().unwrap();
}

#[test]
fn limits_and_tls_boundaries_are_explicit() {
    let config = TcpClientConfig {
        max_request_body_bytes: 2,
        ..TcpClientConfig::default()
    };
    let client = TcpHttpClient::new(config).unwrap();
    let request =
        Request::new(Method::Post, Url::parse("http://127.0.0.1/").unwrap()).body(b"long".to_vec());
    assert_eq!(
        client.send(request).unwrap_err().kind(),
        ErrorKind::InvalidRequest
    );
    let secure = Request::new(Method::Get, Url::parse("https://example.com/").unwrap());
    assert_eq!(
        TcpHttpClient::default().send(secure).unwrap_err().kind(),
        ErrorKind::UnsupportedScheme
    );
}

fn server_once(response: &'static [u8]) -> (Url, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut byte = [0_u8; 1];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        stream.write_all(response).unwrap();
    });
    (
        Url::parse(&format!("http://{address}/test")).unwrap(),
        handle,
    )
}
