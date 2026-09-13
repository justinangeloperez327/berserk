#![cfg(feature = "auth")]

use framework::{
    auth::{Guard, Principal, Result as AuthResult},
    App, Authenticated, Headers, Method, Request, Response,
};

struct TestGuard;
impl Guard for TestGuard {
    fn authenticate(&self, token: &str, _now: u64) -> AuthResult<Option<Principal>> {
        if token == "accepted-token" {
            Ok(Principal::new("user:7"))
        } else {
            Ok(None)
        }
    }
}

fn request(authorization: Option<&str>) -> Request {
    let mut headers = Headers::new();
    if let Some(value) = authorization {
        headers.insert("authorization", value).unwrap();
    }
    Request::new(Method::new("GET").unwrap(), "/private", headers, vec![]).unwrap()
}

#[test]
fn authenticated_middleware_attaches_the_principal() {
    let mut app = App::new();
    app.middleware(Authenticated::new(TestGuard));
    app.get("/private", |request: Request| {
        Response::text(request.principal().unwrap().subject())
    })
    .unwrap();
    let response = app.handle(request(Some("Bearer accepted-token"))).unwrap();
    assert_eq!(response.status_code(), 200);
    assert_eq!(response.body(), b"user:7");
}

#[test]
fn authenticated_middleware_rejects_missing_and_malformed_credentials() {
    let mut app = App::new();
    app.middleware(Authenticated::new(TestGuard));
    app.get("/private", |_request: Request| Response::text("private"))
        .unwrap();

    for authorization in [None, Some("Basic abc"), Some("Bearer bad extra")] {
        let response = app.handle(request(authorization)).unwrap();
        assert_eq!(response.status_code(), 401);
        assert_eq!(response.headers().get("www-authenticate"), Some("Bearer"));
        assert_eq!(response.body(), b"Unauthorized");
    }
}
