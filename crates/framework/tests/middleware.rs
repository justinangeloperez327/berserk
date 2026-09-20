use berserk::{App, Headers, Method, Next, Request, RequestId, Response, Result};
use std::sync::{Arc, Mutex};
fn req(path: &str) -> Request {
    Request::new(
        Method::new("GET").unwrap(),
        path,
        Headers::new(),
        Vec::new(),
    )
    .unwrap()
}
#[test]
fn layers_wrap_in_registration_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut app = App::new();
    for (before, after) in [("a-in", "a-out"), ("b-in", "b-out")] {
        let log = log.clone();
        app.middleware(
            move |request: Request, next: Next<'_>| -> Result<Response> {
                log.lock().unwrap().push(before);
                let response = next.run(request)?;
                log.lock().unwrap().push(after);
                Ok(response)
            },
        );
    }
    let events = log.clone();
    app.route().get("/", move || {
        events.lock().unwrap().push("handler");
        Response::empty()
    })
    .unwrap();
    app.handle(req("/")).unwrap();
    assert_eq!(
        *log.lock().unwrap(),
        vec!["a-in", "b-in", "handler", "b-out", "a-out"]
    );
}
#[test]
fn early_response_and_head_apply_to_whole_pipeline() {
    let mut app = App::new();
    app.middleware(|_: Request, _: Next<'_>| -> Result<Response> {
        Ok(Response::text("blocked").status(403))
    });
    app.route().get("/", || -> Response { panic!("must not run") })
        .unwrap();
    let request = Request::new(
        Method::new("HEAD").unwrap(),
        "/",
        Headers::new(),
        Vec::new(),
    )
    .unwrap();
    let response = app.handle(request).unwrap();
    assert_eq!(response.status_code(), 403);
    assert!(response.body().is_empty());
    assert_eq!(response.representation_length(), 7);
}
#[test]
fn state_and_ids_are_available_and_groups_are_isolated() {
    let mut app = App::new();
    app.state(String::from("shared")).unwrap();
    assert!(app.state(String::from("duplicate")).is_err());
    app.middleware(RequestId);
    app.group("/api", |group| {
        group.middleware(|req: Request, next: Next<'_>| -> Result<Response> {
            next.run(req)?.header("x-group", "api")
        });
        group.route().get("/users/{id}", |req: Request| {
            assert!(req.request_id().is_some());
            Response::text(format!(
                "{}:{}",
                req.state::<String>().unwrap(),
                req.param("id").unwrap()
            ))
        })
    })
    .unwrap();
    app.route().get("/outside", Response::empty).unwrap();
    let response = app.handle(req("/api/users/9")).unwrap();
    assert_eq!(response.body(), b"shared:9");
    assert_eq!(response.headers().get("x-group"), Some("api"));
    let outside = app.handle(req("/outside")).unwrap();
    assert!(outside.headers().get("x-group").is_none());
    assert_ne!(
        response.headers().get("x-request-id"),
        outside.headers().get("x-request-id")
    );
    assert!(app
        .handle(req("/missing"))
        .unwrap()
        .headers()
        .get("x-request-id")
        .is_some());
}
#[test]
fn groups_are_transactional_and_can_nest() {
    let mut app = App::new();
    app.route().get("/api/taken", Response::empty).unwrap();
    assert!(app
        .group("/api", |g| {
            g.route().get("/new", Response::empty)?;
            g.route().get("/taken", Response::empty)
        })
        .is_err());
    assert_eq!(app.handle(req("/api/new")).unwrap().status_code(), 404);
    app.group("/api", |g| {
        g.group("/v1", |v| v.route().get("/ok", || Response::text("ok")))
    })
    .unwrap();
    assert_eq!(app.handle(req("/api/v1/ok")).unwrap().body(), b"ok");
    assert!(app.group("/bad", |g| g.state(1u32)).is_err());
}
