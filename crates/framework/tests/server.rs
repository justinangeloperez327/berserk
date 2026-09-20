use berserk::{App, Request, Response};
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

#[test]
fn serves_real_connections_and_shuts_down() {
    let mut app = App::new();
    app.route()
        .get("/hello/{id}", |req: Request| {
            Response::text(req.param("id").unwrap())
        })
        .unwrap();
    app.route()
        .get("/panic", || -> Response { panic!("test handler panic") })
        .unwrap();
    let server = app.bind("127.0.0.1:0").unwrap();
    let address = server.local_addr().unwrap();
    let shutdown = server.shutdown_handle();
    let worker = std::thread::spawn(move || server.run());
    // Ensure cleanup even if an assertion fails while this guard is alive.
    struct Guard(berserk::ShutdownHandle);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.shutdown();
        }
    }
    let guard = Guard(shutdown);
    for (path, status) in [
        ("/hello/42", "200"),
        ("/missing", "404"),
        ("/panic", "500"),
        ("/hello/43", "200"),
    ] {
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut output = String::new();
        stream.read_to_string(&mut output).unwrap();
        assert!(output.starts_with(&format!("HTTP/1.1 {status}")));
        assert!(output.contains("connection: close\r\n"));
    }
    drop(guard);
    worker.join().unwrap().unwrap();
}

#[test]
fn binding_failure_is_reported_and_shutdown_before_run_works() {
    let server = App::new().bind("127.0.0.1:0").unwrap();
    let address = server.local_addr().unwrap();
    assert!(App::new().bind(address).is_err());
    server.shutdown_handle().shutdown();
    server.run().unwrap();
}
