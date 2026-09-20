use berserk::server::ServerStats;
use berserk::{App, Response, ServerConfig, ShutdownHandle};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Condvar, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
struct Running {
    address: SocketAddr,
    shutdown: ShutdownHandle,
    stats: ServerStats,
    thread: Option<thread::JoinHandle<std::io::Result<()>>>,
}
impl Running {
    fn new(app: App) -> Self {
        let server = app.bind("127.0.0.1:0").unwrap();
        Self {
            address: server.local_addr().unwrap(),
            shutdown: server.shutdown_handle(),
            stats: server.stats(),
            thread: Some(thread::spawn(move || server.run())),
        }
    }
    fn connect(&self) -> TcpStream {
        let stream = TcpStream::connect_timeout(&self.address, Duration::from_secs(2)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
    }
    fn stop(mut self) {
        self.shutdown.shutdown();
        self.thread.take().unwrap().join().unwrap().unwrap();
    }
}
impl Drop for Running {
    fn drop(&mut self) {
        self.shutdown.shutdown();
    }
}
fn wait_for(mut predicate: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(3);
    while !predicate() {
        assert!(Instant::now() < until, "condition timed out");
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn idle_connection_expires_and_worker_remains_available() {
    let mut app = App::with_config(ServerConfig {
        workers: 1,
        read_timeout: Duration::from_millis(100),
        request_deadline: Duration::from_millis(250),
        ..ServerConfig::default()
    })
    .unwrap();
    app.route().get("/", || Response::text("alive")).unwrap();
    let server = Running::new(app);
    let _idle = server.connect();
    wait_for(|| server.stats.snapshot().failed >= 1);
    let mut stream = server.connect();
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n")
        .unwrap();
    let mut result = String::new();
    stream.read_to_string(&mut result).unwrap();
    assert!(result.ends_with("alive"));
    server.stop();
}
#[test]
fn stalled_request_body_expires_and_worker_remains_available() {
    let mut app = App::with_config(ServerConfig {
        workers: 1,
        read_timeout: Duration::from_millis(100),
        request_deadline: Duration::from_millis(500),
        ..ServerConfig::default()
    })
    .unwrap();
    app.route()
        .post("/", || Response::text("unexpected"))
        .unwrap();
    app.route()
        .get("/alive", || Response::text("alive"))
        .unwrap();
    let server = Running::new(app);

    let mut stalled = server.connect();
    stalled
        .write_all(b"POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 5\r\n\r\na")
        .unwrap();
    wait_for(|| server.stats.snapshot().failed >= 1);

    let mut stream = server.connect();
    stream
        .write_all(b"GET /alive HTTP/1.1\r\nHost: x\r\n\r\n")
        .unwrap();
    let mut result = String::new();
    stream.read_to_string(&mut result).unwrap();
    assert!(result.ends_with("alive"));
    server.stop();
}
#[test]
fn oversized_live_body_returns_413_without_invoking_handler() {
    let calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = Arc::clone(&calls);
    let mut app = App::with_config(ServerConfig {
        max_body_bytes: 4,
        ..ServerConfig::default()
    })
    .unwrap();
    app.route()
        .post("/", move || {
            handler_calls.fetch_add(1, Ordering::SeqCst);
            Response::text("unexpected")
        })
        .unwrap();
    let server = Running::new(app);

    let mut stream = server.connect();
    stream
        .write_all(b"POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 5\r\n\r\nhello")
        .unwrap();
    let mut result = String::new();
    stream.read_to_string(&mut result).unwrap();
    assert!(result.starts_with("HTTP/1.1 413"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    server.stop();
}
#[test]
fn queued_first_request_expires_from_accept_time() {
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let blocking_gate = Arc::clone(&gate);
    let (entered, received) = mpsc::channel();
    let queued_calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = Arc::clone(&queued_calls);

    let mut app = App::with_config(ServerConfig {
        workers: 1,
        queue_capacity: 1,
        read_timeout: Duration::from_millis(500),
        request_deadline: Duration::from_millis(100),
        ..ServerConfig::default()
    })
    .unwrap();
    app.route()
        .get("/block", move || {
            let _ = entered.send(());
            let lock = blocking_gate.0.lock().unwrap();
            let (_lock, _) = blocking_gate
                .1
                .wait_timeout_while(lock, Duration::from_secs(3), |ready| !*ready)
                .unwrap();
            Response::text("done")
        })
        .unwrap();
    app.route()
        .get("/queued", move || {
            handler_calls.fetch_add(1, Ordering::SeqCst);
            Response::text("late")
        })
        .unwrap();

    let server = Running::new(app);
    let mut first = server.connect();
    first
        .write_all(b"GET /block HTTP/1.1\r\nHost: x\r\n\r\n")
        .unwrap();
    received.recv_timeout(Duration::from_secs(2)).unwrap();

    let mut queued = server.connect();
    queued
        .write_all(b"GET /queued HTTP/1.1\r\nHost: x\r\n\r\n")
        .unwrap();
    wait_for(|| server.stats.snapshot().accepted >= 2);
    thread::sleep(Duration::from_millis(200));
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();

    let mut first_response = String::new();
    first.read_to_string(&mut first_response).unwrap();
    assert!(first_response.ends_with("done"));
    wait_for(|| server.stats.snapshot().failed >= 1);

    let mut queued_response = Vec::new();
    match queued.read_to_end(&mut queued_response) {
        Ok(_) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ) => {}
        Err(error) => panic!("unexpected queued connection error: {error}"),
    }
    assert!(queued_response.is_empty());
    assert_eq!(queued_calls.load(Ordering::SeqCst), 0);
    server.stop();
}
#[test]
fn full_queue_rejects_and_shutdown_drains_accepted_work() {
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    struct Release(Arc<(Mutex<bool>, Condvar)>);
    impl Drop for Release {
        fn drop(&mut self) {
            *self.0 .0.lock().unwrap() = true;
            self.0 .1.notify_all();
        }
    }
    let release = Release(gate.clone());
    let (entered, received) = mpsc::channel();
    let mut app = App::with_config(ServerConfig {
        workers: 1,
        queue_capacity: 1,
        ..ServerConfig::default()
    })
    .unwrap();
    app.route()
        .get("/", move || {
            let _ = entered.send(());
            let lock = gate.0.lock().unwrap();
            let (_lock, _) = gate
                .1
                .wait_timeout_while(lock, Duration::from_secs(3), |ready| !*ready)
                .unwrap();
            Response::text("done")
        })
        .unwrap();
    let server = Running::new(app);
    let mut first = server.connect();
    first
        .write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n")
        .unwrap();
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    let mut second = server.connect();
    second
        .write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n")
        .unwrap();
    wait_for(|| server.stats.snapshot().accepted >= 2);
    let mut third = server.connect();
    wait_for(|| server.stats.snapshot().rejected == 1);
    let mut byte = [0];
    match third.read(&mut byte) {
        Ok(0) => {}
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ) => {}
        other => panic!("expected overload close: {other:?}"),
    }
    server.shutdown.shutdown();
    drop(release);
    for stream in [&mut first, &mut second] {
        let mut out = String::new();
        stream.read_to_string(&mut out).unwrap();
        assert!(out.ends_with("done"));
    }
    let stats = server.stats.clone();
    server.stop();
    assert_eq!(stats.snapshot().completed, 2);
}
#[test]
fn truncated_client_does_not_consume_worker_permanently() {
    let mut app = App::new();
    app.route().get("/", Response::empty).unwrap();
    let server = Running::new(app);
    let mut stream = server.connect();
    stream
        .write_all(b"POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 5\r\n\r\na")
        .unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    wait_for(|| server.stats.snapshot().failed >= 1);
    server.stop();
}
