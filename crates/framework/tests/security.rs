use berserk::{Headers, Method, Request, Response};

#[test]
fn request_debug_redacts_target_headers_query_and_body() {
    let mut headers = Headers::new();
    headers
        .append("authorization", "Bearer header-secret")
        .unwrap();
    let request = Request::new(
        Method::new("GET").unwrap(),
        "/users/route-secret?token=query-secret",
        headers,
        b"body-secret".to_vec(),
    )
    .unwrap();

    let debug = format!("{request:?}");
    assert!(debug.contains("target_bytes"));
    for secret in [
        "/users",
        "route-secret",
        "header-secret",
        "query-secret",
        "body-secret",
    ] {
        assert!(!debug.contains(secret));
    }
}

#[test]
fn response_debug_redacts_headers_and_body() {
    let response = Response::text("response-secret")
        .header("set-cookie", "session=cookie-secret")
        .unwrap();
    let debug = format!("{response:?}");
    assert!(debug.contains("body_bytes"));
    assert!(!debug.contains("response-secret"));
    assert!(!debug.contains("cookie-secret"));
}
