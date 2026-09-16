use framework::{
    server::{read_request, write_response, ProtocolError},
    App, Method, Response, ServerConfig,
};
use std::io::{self, Cursor, Read, Write};

fn parse(bytes: &[u8]) -> Result<framework::Request, ProtocolError> {
    read_request(&mut Cursor::new(bytes), &ServerConfig::default())
}
struct Fragmented<'a>(&'a [u8]);
impl Read for Fragmented<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() || self.0.is_empty() {
            return Ok(0);
        }
        out[0] = self.0[0];
        self.0 = &self.0[1..];
        Ok(1)
    }
}
#[test]
fn partial_reads_and_exact_body_consumption() {
    let input = b"POST /echo?q=1 HTTP/1.1\r\nHost: localhost\r\nContent-Length: 3\r\n\r\nabcNEXT";
    let mut reader = Fragmented(input);
    let req = read_request(&mut reader, &ServerConfig::default()).unwrap();
    assert_eq!(req.body(), b"abc");
    assert_eq!(req.query_string(), Some("q=1"));
    assert_eq!(reader.0, b"NEXT");
}
#[test]
fn rejects_ambiguous_or_unsupported_framing_before_body_read() {
    for headers in [
        "Content-Length: 1\r\nContent-Length: 1\r\n",
        "Content-Length: 1\r\nContent-Length: 2\r\n",
        "Content-Length: 1, 1\r\n",
        "Content-Length: +1\r\n",
        "Content-Length: -1\r\n",
        "Content-Length: \r\n",
        "Transfer-Encoding: chunked\r\nContent-Length: 1\r\n",
    ] {
        let text = format!("POST / HTTP/1.1\r\nHost: x\r\n{headers}\r\n");
        assert!(
            matches!(parse(text.as_bytes()), Err(ProtocolError::Malformed)),
            "{headers}"
        );
    }
    assert!(matches!(
        parse(b"POST / HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: gzip\r\n\r\n"),
        Err(ProtocolError::UnsupportedTransferEncoding)
    ));
    assert!(matches!(
        parse(b"POST / HTTP/1.1\r\nHost: x\r\nExpect: 100-continue\r\n\r\n"),
        Err(ProtocolError::UnsupportedExpectation)
    ));
}
#[test]
fn host_and_header_grammar_are_strict() {
    for host in ["localhost", "example.com:80", "[::1]", "[::1]:3000"] {
        assert!(parse(format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes()).is_ok());
    }
    for host in ["", "a b", "a:bad", "a:65536", "a@b", "[oops]", "::1", "a/b"] {
        assert!(parse(format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes()).is_err());
    }
    for wire in [
        "GET / HTTP/1.1\r\n\r\n",
        "GET / HTTP/1.1\r\nHost: x\r\nHost: y\r\n\r\n",
        "GET / HTTP/1.1\r\nHost : x\r\n\r\n",
        "GET / HTTP/1.1\r\nHost: x\r\n folded\r\n\r\n",
        "GET  / HTTP/1.1\r\nHost: x\r\n\r\n",
        "GET /#x HTTP/1.1\r\nHost: x\r\n\r\n",
    ] {
        assert!(parse(wire.as_bytes()).is_err(), "{wire}");
    }
}
#[test]
fn limits_and_truncation_are_enforced() {
    let wire = b"GET / HTTP/1.1\r\nHost: x\r\n\r\n";
    let mut config = ServerConfig {
        max_header_bytes: wire.len(),
        ..ServerConfig::default()
    };
    assert!(read_request(&mut Cursor::new(wire), &config).is_ok());
    config.max_header_bytes -= 1;
    assert!(matches!(
        read_request(&mut Cursor::new(wire), &config),
        Err(ProtocolError::HeaderLimit)
    ));
    config = ServerConfig {
        max_headers: 1,
        ..ServerConfig::default()
    };
    assert!(matches!(
        read_request(
            &mut Cursor::new(b"GET / HTTP/1.1\r\nHost: x\r\nX: a\r\n\r\n"),
            &config
        ),
        Err(ProtocolError::HeaderLimit)
    ));
    config = ServerConfig {
        max_body_bytes: 2,
        ..ServerConfig::default()
    };
    assert!(matches!(
        read_request(
            &mut Cursor::new(b"POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 3\r\n\r\n"),
            &config
        ),
        Err(ProtocolError::BodyLimit)
    ));
    assert!(matches!(
        parse(b"POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 3\r\n\r\na"),
        Err(ProtocolError::Io(_))
    ));
    assert!(parse(b"GET / HTTP/1.1\r\nHost: x").is_err());
    assert!(matches!(
        parse(b"GET / HTTP/1.0\r\nHost: x\r\n\r\n"),
        Err(ProtocolError::UnsupportedVersion)
    ));
}
fn encode(response: &Response, method: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_response(&mut bytes, response, &Method::new(method).unwrap()).unwrap();
    bytes
}
#[test]
fn response_lengths_head_and_bodyless_statuses() {
    let wire = encode(&Response::text("hé"), "GET");
    assert!(wire.ends_with("\r\n\r\nhé".as_bytes()));
    assert!(String::from_utf8_lossy(&wire).contains("content-length: 3\r\n"));
    for code in [204, 304] {
        let wire = String::from_utf8(encode(&Response::empty().status(code), "GET")).unwrap();
        assert!(!wire.contains("content-length"));
        assert!(wire.ends_with("\r\n\r\n"));
    }
    let wire = String::from_utf8(encode(&Response::empty().status(205), "GET")).unwrap();
    assert!(wire.contains("content-length: 0\r\n"));
    let mut app = App::new();
    app.get("/", || Response::text("hello")).unwrap();
    let request = parse(b"HEAD / HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
    let response = app.handle(request).unwrap();
    let wire = String::from_utf8(encode(&response, "HEAD")).unwrap();
    assert!(wire.contains("content-length: 5\r\n"));
    assert!(wire.ends_with("\r\n\r\n"));
}
#[test]
fn invalid_response_writes_nothing_and_transport_failures_propagate() {
    let mut output = Vec::new();
    assert!(write_response(
        &mut output,
        &Response::text("bad").status(204),
        &Method::new("GET").unwrap()
    )
    .is_err());
    assert!(output.is_empty());
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert!(matches!(
        write_response(
            &mut Broken,
            &Response::empty(),
            &Method::new("GET").unwrap()
        ),
        Err(ProtocolError::Io(_))
    ));
}
