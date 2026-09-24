use berserk::{App, Headers, Method, Request, Response};

fn request(method: &str, path: &str) -> Request {
    Request::new(
        Method::new(method).unwrap(),
        path,
        Headers::new(),
        Vec::new(),
    )
    .unwrap()
}

#[test]
fn allow_header_is_derived_from_the_selected_route_pattern() {
    let mut app = App::new();
    app.route()
        .get("/items/{id}", |_id: u64| Response::empty())
        .unwrap();
    app.route()
        .put("/items/{id}", |_id: u64| Response::empty())
        .unwrap();
    app.route()
        .post("/items/new", || Response::empty())
        .unwrap();

    let dynamic = app.respond(request("POST", "/items/7"));
    assert_eq!(dynamic.status_code(), 405);
    assert_eq!(dynamic.headers().get("allow"), Some("GET, HEAD, PUT"));

    let static_route = app.respond(request("GET", "/items/new"));
    assert_eq!(static_route.status_code(), 405);
    assert_eq!(static_route.headers().get("allow"), Some("POST"));
}

#[test]
fn explicit_head_wins_over_get_fallback() {
    let mut app = App::new();
    app.route().get("/x", || Response::text("get")).unwrap();
    app.route().head("/x", || Response::text("head")).unwrap();

    let response = app.respond(request("HEAD", "/x"));
    assert_eq!(response.status_code(), 200);
    assert!(response.body().is_empty());
    assert_eq!(response.representation_length(), 4);
}

#[test]
fn query_string_does_not_participate_in_route_matching() {
    let mut app = App::new();
    app.route()
        .get("/search", |request: Request| {
            Response::text(request.query("q").unwrap().unwrap_or_default())
        })
        .unwrap();
    let response = app.respond(request("GET", "/search?q=rust"));
    assert_eq!(response.status_code(), 200);
    assert_eq!(response.body(), b"rust");
}
