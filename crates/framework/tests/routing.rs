use framework::{App, Error, Headers, HttpError, Method, Request, Response, Result, RouteError};

fn request(method: &str, path: &str) -> Request {
    Request::new(
        Method::new(method).unwrap(),
        path,
        Headers::new(),
        Vec::new(),
    )
    .unwrap()
}
fn named(req: Request) -> Response {
    Response::text(req.param("id").unwrap_or("missing"))
}

#[test]
fn named_inline_and_fallible_handlers_work() {
    let mut app = App::new();
    app.get("/users/{id}", named).unwrap();
    app.post("/users/{key}", |req| {
        Response::text(req.param("key").unwrap())
    })
    .unwrap();
    app.get("/fail", |_| -> Result<Response> {
        Err(HttpError::InvalidTarget.into())
    })
    .unwrap();
    assert_eq!(
        app.handle(request("GET", "/users/a%2Fb?q=1"))
            .unwrap()
            .body(),
        b"a%2Fb"
    );
    assert_eq!(
        app.handle(request("POST", "/users/42")).unwrap().body(),
        b"42"
    );
    assert!(matches!(
        app.handle(request("GET", "/fail")),
        Err(Error::Http(HttpError::InvalidTarget))
    ));
}

#[test]
fn precedence_is_independent_of_order_and_precedes_method_selection() {
    for reversed in [false, true] {
        let mut app = App::new();
        if reversed {
            app.post("/users/new", |_| Response::text("static"))
                .unwrap();
        }
        app.get("/users/{id}", named).unwrap();
        if !reversed {
            app.post("/users/new", |_| Response::text("static"))
                .unwrap();
        }
        let response = app.handle(request("GET", "/users/new")).unwrap();
        assert_eq!(response.status_code(), 405);
        assert_eq!(response.headers().get("allow"), Some("POST"));
        assert_eq!(
            app.handle(request("POST", "/users/new")).unwrap().body(),
            b"static"
        );
    }
    let mut app = App::new();
    app.get("/{x}/fixed", |_| Response::text("later static"))
        .unwrap();
    app.get("/fixed/{x}", |_| Response::text("earlier static"))
        .unwrap();
    assert_eq!(
        app.handle(request("GET", "/fixed/fixed")).unwrap().body(),
        b"earlier static"
    );
}

#[test]
fn invalid_registration_is_atomic() {
    let mut app = App::new();
    app.get("/users/{id}", named).unwrap();
    assert!(matches!(
        app.get("/users/{other}", named),
        Err(Error::Routing(RouteError::DuplicateRoute))
    ));
    for path in [
        "users",
        "/{id}/{id}",
        "/{}",
        "/{1id}",
        "/{id",
        "/x{id}",
        "/a?b",
        "/a#b",
        "/a*",
        "/%GG",
        "/a b",
    ] {
        assert!(app.get(path, named).is_err(), "{path}");
    }
    assert_eq!(app.handle(request("GET", "/users/9")).unwrap().body(), b"9");
}

#[test]
fn missing_methods_and_trailing_slashes_are_distinct() {
    let mut app = App::new();
    app.get("/", |_| Response::text("root")).unwrap();
    app.get("/x", |_| Response::text("plain")).unwrap();
    app.get("/x/", |_| Response::text("slash")).unwrap();
    app.put("/x", |_| Response::empty()).unwrap();
    assert_eq!(app.handle(request("GET", "/x/")).unwrap().body(), b"slash");
    assert_eq!(
        app.handle(request("GET", "/missing"))
            .unwrap()
            .status_code(),
        404
    );
    let response = app.handle(request("OPTIONS", "/x")).unwrap();
    assert_eq!(response.status_code(), 405);
    assert_eq!(response.headers().get("allow"), Some("GET, HEAD, PUT"));
    app.options("/x", |_| Response::empty().status(204))
        .unwrap();
    assert_eq!(
        app.handle(request("OPTIONS", "/x")).unwrap().status_code(),
        204
    );
}

#[test]
fn head_suppresses_all_successfully_dispatched_bodies() {
    let mut app = App::new();
    app.get("/x", |_| Response::text("hé")).unwrap();
    let response = app.handle(request("HEAD", "/x")).unwrap();
    assert!(response.body().is_empty());
    assert_eq!(response.representation_length(), 3);
    app.head("/x", |_| Response::text("explicit")).unwrap();
    let response = app.handle(request("HEAD", "/x")).unwrap();
    assert!(response.body().is_empty());
    assert_eq!(response.representation_length(), 8);
    assert_eq!(
        app.handle(request("GET", "/x")).unwrap().body(),
        "hé".as_bytes()
    );
    assert!(app
        .handle(request("HEAD", "/missing"))
        .unwrap()
        .body()
        .is_empty());
    app.post("/post", |_| Response::empty()).unwrap();
    let response = app.handle(request("HEAD", "/post")).unwrap();
    assert_eq!(response.status_code(), 405);
    assert!(response.body().is_empty());
}

#[test]
fn parameters_require_nonempty_segments_and_invalid_responses_propagate() {
    let mut app = App::new();
    app.get("/p/{id}", named).unwrap();
    app.get("/bad", |_| Response::text("invalid").status(204))
        .unwrap();
    assert_eq!(
        app.handle(request("GET", "/p/")).unwrap().status_code(),
        404
    );
    assert_eq!(
        app.handle(request("GET", "/p/a/b")).unwrap().status_code(),
        404
    );
    assert!(app.handle(request("HEAD", "/bad")).is_err());
}

#[test]
fn app_accepts_thread_safe_captures_and_concurrent_dispatch() {
    let counter = framework::State::new(std::sync::atomic::AtomicUsize::new(0));
    let captured = counter.clone();
    let mut app = App::new();
    app.get("/", move |_| {
        captured.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Response::empty()
    })
    .unwrap();
    let app = std::sync::Arc::new(app);
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let app = app.clone();
            std::thread::spawn(move || app.handle(request("GET", "/")).unwrap())
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 4);
}
