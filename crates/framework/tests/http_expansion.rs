use framework::{App, Headers, Method, Request, Response, ServerConfig};
use std::{
    io::{Cursor, Read, Write},
    net::TcpStream,
    time::Duration,
};

struct Stop(framework::ShutdownHandle);
impl Drop for Stop {
    fn drop(&mut self) {
        self.0.shutdown();
    }
}

fn request(addr: std::net::SocketAddr, wire: &[u8]) -> String {
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream.write_all(wire).unwrap();
    let mut out = String::new();
    stream.read_to_string(&mut out).unwrap();
    out
}

#[test]
fn chunked_request_body_reaches_handler() {
    let mut app = App::new();
    app.post("/echo", |request: Request| {
        Response::text(String::from_utf8_lossy(request.body()).into_owned())
    })
    .unwrap();
    let server = app.bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap();
    let stop = Stop(server.shutdown_handle());
    let worker = std::thread::spawn(move || server.run());

    let out = request(
        addr,
        b"POST /echo HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nhi\r\n0\r\n\r\n",
    );
    assert!(out.starts_with("HTTP/1.1 200"));
    assert!(out.ends_with("hi"));

    drop(stop);
    worker.join().unwrap().unwrap();
}

#[test]
fn streaming_and_head_work_over_live_transport() {
    let mut app = App::new();
    app.get("/stream", || Response::stream(Cursor::new(b"abc".to_vec())))
        .unwrap();
    app.get("/head", || Response::text("hello")).unwrap();
    let server = app.bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap();
    let stop = Stop(server.shutdown_handle());
    let worker = std::thread::spawn(move || server.run());

    let streamed = request(addr, b"GET /stream HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(streamed.starts_with("HTTP/1.1 200"));
    assert!(streamed.contains("transfer-encoding: chunked\r\n"));
    assert!(streamed.contains("\r\n3\r\nabc\r\n0\r\n\r\n"));

    let head = request(addr, b"HEAD /head HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(head.starts_with("HTTP/1.1 200"));
    assert!(head.contains("content-length: 5\r\n"));
    assert!(head.ends_with("\r\n\r\n"));

    drop(stop);
    worker.join().unwrap().unwrap();
}

#[test]
fn multipart_part_and_limit() {
    let mut headers = Headers::new();
    headers
        .insert("content-type", "multipart/form-data; boundary=test")
        .unwrap();
    let req = Request::new(
        Method::new("POST").unwrap(),
        "/",
        headers,
        b"--test\r\nContent-Disposition: form-data; name=note\r\n\r\nhello\r\n--test--\r\n"
            .to_vec(),
    )
    .unwrap();
    assert_eq!(req.multipart(2, 1024).unwrap()[0].body, b"hello");
    assert!(req.multipart(0, 1024).is_err());
}

#[test]
fn keep_alive_is_bounded() {
    let mut app = App::with_config(ServerConfig {
        keep_alive: true,
        max_requests_per_connection: 2,
        ..ServerConfig::default()
    })
    .unwrap();
    app.get("/", || Response::text("ok")).unwrap();
    let server = app.bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap();
    let stop = Stop(server.shutdown_handle());
    let worker = std::thread::spawn(move || server.run());
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\nGET / HTTP/1.1\r\nHost: x\r\n\r\n")
        .unwrap();
    let mut out = String::new();
    stream.read_to_string(&mut out).unwrap();
    assert_eq!(out.matches("HTTP/1.1 200").count(), 2);
    assert!(out.contains("connection: keep-alive"));
    assert!(out.contains("connection: close"));
    drop(stop);
    worker.join().unwrap().unwrap();
}
