use framework::server::{read_request, write_response};
use framework::{App, Headers, Method, Request, Response, ServerConfig};
use std::io::Cursor;
#[test]
fn chunked_body_preserves_next_request_and_limits() {
    let mut input = Cursor::new(
        b"POST / HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nhi\r\n0\r\n\r\nNEXT",
    );
    let request = read_request(&mut input, &ServerConfig::default()).unwrap();
    assert_eq!(request.body(), b"hi");
    assert_eq!(&input.get_ref()[input.position() as usize..], b"NEXT");
    input.set_position(0);
    let config = ServerConfig {
        max_body_bytes: 1,
        ..ServerConfig::default()
    };
    assert!(read_request(&mut input, &config).is_err());
}
#[test]
fn stream_chunks_and_head_do_not_consume_reader() {
    let response = Response::stream(Cursor::new(b"abc".to_vec()));
    let mut head = Vec::new();
    write_response(&mut head, &response, &Method::new("HEAD").unwrap()).unwrap();
    assert!(head.ends_with(b"\r\n\r\n"));
    let mut out = Vec::new();
    write_response(&mut out, &response, &Method::new("GET").unwrap()).unwrap();
    assert!(out.ends_with(b"3\r\nabc\r\n0\r\n\r\n"));
    assert!(write_response(&mut Vec::new(), &response, &Method::new("GET").unwrap()).is_err());
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
    use std::{
        io::{Read, Write},
        net::TcpStream,
        time::Duration,
    };
    let mut app = App::with_config(ServerConfig {
        keep_alive: true,
        max_requests_per_connection: 2,
        ..ServerConfig::default()
    })
    .unwrap();
    app.get("/", || Response::text("ok")).unwrap();
    let server = app.bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap();
    struct Stop(framework::ShutdownHandle);
    impl Drop for Stop {
        fn drop(&mut self) {
            self.0.shutdown();
        }
    }
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
