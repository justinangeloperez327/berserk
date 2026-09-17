use berserk::{Headers, HttpError, IntoResponse, Method, Request, Response, StatusCode};

#[test]
fn method_tokens_are_validated_and_case_sensitive() {
    for method in [
        "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "CUSTOM",
    ] {
        assert_eq!(Method::new(method).unwrap().as_str(), method);
    }
    for method in ["", "GET HTTP/1.1", "GET\r\n", "méthod"] {
        assert!(Method::new(method).is_err());
    }
    assert_ne!(Method::new("get").unwrap(), Method::new("GET").unwrap());
}

#[test]
fn headers_preserve_repetitions_and_replace_atomically() {
    let mut h = Headers::new();
    h.append("X-Test", " one ").unwrap();
    h.append("x-test", "two").unwrap();
    assert_eq!(h.get_all("X-TEST").collect::<Vec<_>>(), vec!["one", "two"]);
    let before = h.clone();
    assert!(h.insert("x-test", "bad\r\nInjected: yes").is_err());
    assert_eq!(h, before);
    h.insert("X-Test", "three").unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(h.get("x-test"), Some("three"));
    for name in ["", "x y", "x:", "é"] {
        assert!(h.append(name, "value").is_err());
    }
    for value in ["\0", "\n", "\r", "\u{7f}", "é"] {
        assert!(h.append("x", value).is_err());
    }
}

#[test]
fn request_preserves_raw_components_and_owns_body() {
    let req = Request::new(
        Method::new("POST").unwrap(),
        "/users/a%2Fb?q=a+b&x=%20",
        Headers::new(),
        vec![0xff],
    )
    .unwrap();
    assert_eq!(req.path(), "/users/a%2Fb");
    assert_eq!(req.query_string(), Some("q=a+b&x=%20"));
    assert_eq!(req.body(), &[0xff]);
    assert!(req.text().is_err());
    assert_eq!(req.param("id"), None);
    for target in [
        "*",
        "https://example.com/",
        "/a b",
        "/#fragment",
        "/%",
        "/%GG",
        "/é",
        "/a\r\n",
    ] {
        assert!(Request::new(
            Method::new("GET").unwrap(),
            target,
            Headers::new(),
            Vec::new()
        )
        .is_err());
    }
    for (target, query) in [("/", None), ("/?", Some("")), ("/?a?b", Some("a?b"))] {
        let req = Request::new(
            Method::new("GET").unwrap(),
            target,
            Headers::new(),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(req.query_string(), query);
    }
}

#[test]
fn request_debug_redacts_target_values() {
    let request = Request::new(
        Method::new("GET").unwrap(),
        "/users/secret-route-value?token=secret-query-value",
        Headers::new(),
        Vec::new(),
    )
    .unwrap();
    let debug = format!("{request:?}");
    assert!(!debug.contains("secret-route-value"));
    assert!(!debug.contains("secret-query-value"));
    assert!(debug.contains("target_bytes"));
}

#[test]
fn text_response_outlives_request_and_counts_bytes() {
    let response = {
        let req = Request::new(
            Method::new("POST").unwrap(),
            "/",
            Headers::new(),
            "hé".as_bytes().to_vec(),
        )
        .unwrap();
        Response::text(req.text().unwrap())
    };
    assert_eq!(response.body().len(), 3);
    assert_eq!(
        response.headers().get("content-type"),
        Some("text/plain; charset=utf-8")
    );
}

#[test]
fn framing_headers_and_invalid_statuses_are_rejected() {
    for name in ["Content-Length", "TRANSFER-ENCODING", "connection"] {
        assert!(Response::empty().header(name, "0").is_err());
        assert!(Response::empty().append_header(name, "0").is_err());
    }
    for code in [0, 101, 199, 600, u16::MAX] {
        assert!(StatusCode::new(code).is_err());
        assert!(Response::empty().status(code).into_response().is_err());
    }
    for code in [204, 205, 304] {
        assert_eq!(
            Response::text("body").status(code).validate(),
            Err(HttpError::BodyNotAllowed(code))
        );
        assert!(Response::empty().status(code).validate().is_ok());
    }
    let response = Response::empty()
        .append_header("set-cookie", "a=1")
        .unwrap()
        .append_header("Set-Cookie", "b=2")
        .unwrap();
    assert_eq!(response.headers().get_all("set-cookie").count(), 2);
    assert!(Response::text("created")
        .status(201)
        .into_response()
        .is_ok());
    let failure: berserk::Result<Response> = Err(HttpError::InvalidTarget.into());
    assert!(failure.into_response().is_err());
}
