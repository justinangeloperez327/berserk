use berserk::{App, Error, Headers, HttpError, Method, Request, Response, Result, RouteError};

fn request(method: &str, path: &str) -> Request {
    request_with_body(method, path, Vec::new())
}

fn request_with_body(method: &str, path: &str, body: impl Into<Vec<u8>>) -> Request {
    Request::new(Method::new(method).unwrap(), path, Headers::new(), body).unwrap()
}

fn named(req: Request) -> Response {
    Response::text(req.param("id").unwrap_or("missing"))
}

fn typed_show(id: u64) -> Response {
    Response::text(id.to_string())
}

fn typed_store(req: Request) -> Response {
    Response::text(req.text().unwrap_or(""))
}

fn typed_update(id: u64, req: Request) -> Response {
    Response::text(format!("{id}:{}", req.text().unwrap_or("")))
}

#[test]
fn named_inline_and_fallible_handlers_work() {
    let mut app = App::new();
    app.route()
        .get("/users/{id}", named).unwrap();
    app.route()
        .post("/users/{key}", |req: Request| {
        Response::text(req.param("key").unwrap())
    })
    .unwrap();
    app.route()
        .get("/fail", || -> Result<Response> {
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
fn standard_verbs_infer_controller_arguments() {
    let mut app = App::new();
    app.route()
        .get("/users/{id}", typed_show).unwrap();
    app.route()
        .post("/users", typed_store).unwrap();
    app.route()
        .put("/users/{id}", typed_update).unwrap();
    app.route()
        .patch("/users/{id}", typed_update).unwrap();
    app.route()
        .delete("/users/{id}", typed_show).unwrap();

    assert_eq!(
        app.handle(request("GET", "/users/42")).unwrap().body(),
        b"42"
    );
    assert_eq!(
        app.handle(request_with_body("POST", "/users", "created"))
            .unwrap()
            .body(),
        b"created"
    );
    assert_eq!(
        app.handle(request_with_body("PUT", "/users/7", "full"))
            .unwrap()
            .body(),
        b"7:full"
    );
    assert_eq!(
        app.handle(request_with_body("PATCH", "/users/7", "partial"))
            .unwrap()
            .body(),
        b"7:partial"
    );
    assert_eq!(
        app.handle(request("DELETE", "/users/9")).unwrap().body(),
        b"9"
    );
    assert_eq!(
        app.handle(request("GET", "/users/not-a-number"))
            .unwrap()
            .status_code(),
        400
    );
    assert!(matches!(
        app.route()
        .get("/users/{user}/posts/{post}", typed_show),
        Err(Error::Routing(RouteError::ParameterCountMismatch {
            expected: 1,
            actual: 2
        }))
    ));
}

#[test]
fn precedence_is_independent_of_order_and_precedes_method_selection() {
    for reversed in [false, true] {
        let mut app = App::new();
        if reversed {
            app.route()
        .post("/users/new", || Response::text("static")).unwrap();
        }
        app.route()
        .get("/users/{id}", named).unwrap();
        if !reversed {
            app.route()
        .post("/users/new", || Response::text("static")).unwrap();
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
    app.route()
        .get("/{x}/fixed", |_req: Request| Response::text("later static"))
        .unwrap();
    app.route()
        .get("/fixed/{x}", |_req: Request| {
        Response::text("earlier static")
    })
    .unwrap();
    assert_eq!(
        app.handle(request("GET", "/fixed/fixed")).unwrap().body(),
        b"earlier static"
    );
}

#[test]
fn invalid_registration_is_atomic() {
    let mut app = App::new();
    app.route()
        .get("/users/{id}", named).unwrap();
    assert!(matches!(
        app.route()
        .get("/users/{other}", named),
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
        assert!(app.route()
        .get(path, named).is_err(), "{path}");
    }
    assert_eq!(app.handle(request("GET", "/users/9")).unwrap().body(), b"9");
}

#[test]
fn missing_methods_and_trailing_slashes_are_distinct() {
    let mut app = App::new();
    app.route()
        .get("/", || Response::text("root")).unwrap();
    app.route()
        .get("/x", || Response::text("plain")).unwrap();
    app.route()
        .get("/x/", || Response::text("slash")).unwrap();
    app.route()
        .put("/x", Response::empty).unwrap();
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
}

#[test]
fn head_uses_get_and_suppresses_bodies() {
    let mut app = App::new();
    app.route()
        .get("/x", || Response::text("hé")).unwrap();
    let response = app.handle(request("HEAD", "/x")).unwrap();
    assert!(response.body().is_empty());
    assert_eq!(response.representation_length(), 3);
    assert_eq!(
        app.handle(request("GET", "/x")).unwrap().body(),
        "hé".as_bytes()
    );
    assert!(app
        .handle(request("HEAD", "/missing"))
        .unwrap()
        .body()
        .is_empty());
    app.route()
        .post("/post", Response::empty).unwrap();
    let response = app.handle(request("HEAD", "/post")).unwrap();
    assert_eq!(response.status_code(), 405);
    assert!(response.body().is_empty());
}

#[test]
fn parameters_require_nonempty_segments_and_invalid_responses_propagate() {
    let mut app = App::new();
    app.route()
        .get("/p/{id}", named).unwrap();
    app.route()
        .get("/bad", || Response::text("invalid").status(204))
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
    let counter = berserk::State::new(std::sync::atomic::AtomicUsize::new(0));
    let captured = counter.clone();
    let mut app = App::new();
    app.route()
        .get("/", move || {
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
