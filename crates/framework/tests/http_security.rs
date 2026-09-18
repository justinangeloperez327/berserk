use berserk::{App, Cors, Error, Headers, Method, Request, Response, Result, SecurityHeaders};
use std::time::Duration;

fn request(method: &str, headers: &[(&str, &str)]) -> Request {
    let mut fields = Headers::new();
    for (name, value) in headers {
        fields.append(name, value).unwrap();
    }
    Request::new(Method::new(method).unwrap(), "/", fields, vec![]).unwrap()
}

fn application(cors: Cors) -> App {
    let mut app = App::new();
    app.middleware(SecurityHeaders::default());
    app.middleware(cors);
    app.route()
        .get("/", || {
            Response::text("ok").header("vary", "Accept-Encoding")
        })
        .unwrap();
    app.route().post("/", || Response::text("created")).unwrap();
    app
}

fn policy() -> Cors {
    Cors::new()
        .allow_origin("https://client.example")
        .unwrap()
        .allow_methods(["GET", "POST"])
        .unwrap()
        .allow_headers(["Authorization", "Content-Type"])
        .unwrap()
        .expose_headers(["X-Request-Id"])
        .unwrap()
        .max_age(Duration::from_secs(600))
        .unwrap()
}

#[test]
fn origins_are_explicit_and_vary_preserves_existing_cache_keys() {
    let app = application(policy());
    let response = app.respond(request("GET", &[("origin", "https://client.example")]));
    assert_eq!(response.status_code(), 200);
    assert_eq!(
        response.headers().get("access-control-allow-origin"),
        Some("https://client.example")
    );
    assert_eq!(
        response.headers().get("access-control-expose-headers"),
        Some("x-request-id")
    );
    assert_eq!(
        response.headers().get("vary"),
        Some("Accept-Encoding, Origin")
    );
    assert!(response
        .headers()
        .get("access-control-allow-credentials")
        .is_none());
    for origin in [
        "https://attacker.example",
        "https://client.example.attacker.test",
        "http://client.example",
        "null",
    ] {
        let denied = app.respond(request("GET", &[("origin", origin)]));
        assert_eq!(denied.status_code(), 403);
        assert!(denied
            .headers()
            .get("access-control-allow-origin")
            .is_none());
        assert_eq!(
            denied.headers().get("x-content-type-options"),
            Some("nosniff")
        );
        assert_eq!(denied.headers().get("vary"), Some("Origin"));
    }
    let local = app.respond(request("GET", &[]));
    assert_eq!(local.status_code(), 200);
    assert!(local.headers().get("access-control-allow-origin").is_none());
    assert_eq!(local.headers().get("vary"), Some("Accept-Encoding, Origin"));
    let defaults = application(Cors::default());
    assert_eq!(
        defaults
            .respond(request("GET", &[("origin", "https://client.example")]))
            .status_code(),
        403
    );
}

#[test]
fn preflight_checks_methods_and_headers_without_calling_protected_handlers() {
    let app = application(policy());
    let preflight = [
        ("origin", "https://client.example"),
        ("access-control-request-method", "POST"),
        (
            "access-control-request-headers",
            "AUTHORIZATION, content-type",
        ),
    ];
    let response = app.respond(request("OPTIONS", &preflight));
    assert_eq!(response.status_code(), 204);
    assert!(response.body().is_empty());
    assert_eq!(
        response.headers().get("access-control-allow-methods"),
        Some("GET, POST")
    );
    assert_eq!(
        response.headers().get("access-control-allow-headers"),
        Some("authorization, content-type")
    );
    assert_eq!(
        response.headers().get("access-control-max-age"),
        Some("600")
    );
    assert_eq!(
        response.headers().get("vary"),
        Some("Origin, Access-Control-Request-Method, Access-Control-Request-Headers")
    );
    for (method, headers, status) in [
        ("DELETE", "authorization", 403),
        ("POST", "x-private", 403),
        ("post", "authorization", 403),
        ("POST", "authorization,", 400),
        ("POST", "*", 400),
    ] {
        let response = app.respond(request(
            "OPTIONS",
            &[
                ("origin", "https://client.example"),
                ("access-control-request-method", method),
                ("access-control-request-headers", headers),
            ],
        ));
        assert_eq!(response.status_code(), status);
        assert!(response
            .headers()
            .get("access-control-allow-origin")
            .is_none());
    }
    assert_eq!(
        app.respond(request("DELETE", &[("origin", "https://client.example")]))
            .status_code(),
        403
    );
}

#[test]
fn ambiguous_or_malformed_cors_inputs_fail_closed() {
    let app = application(policy());
    for headers in [
        vec![
            ("origin", "https://client.example"),
            ("origin", "https://attacker.test"),
        ],
        vec![("origin", "https://client.example, https://attacker.test")],
        vec![("origin", "https://client.example/path")],
        vec![("access-control-request-method", "POST")],
        vec![
            ("origin", "https://client.example"),
            ("access-control-request-method", "POST"),
            ("access-control-request-method", "GET"),
        ],
        vec![
            ("origin", "https://client.example"),
            ("access-control-request-method", "POST"),
            ("access-control-request-headers", "authorization"),
            ("access-control-request-headers", "content-type"),
        ],
    ] {
        let response = app.respond(request("OPTIONS", &headers));
        assert_eq!(response.status_code(), 400, "{headers:?}");
        assert!(response.headers().get("access-control-allow-origin").is_none());
    }
}

#[test]
fn credentials_and_wildcards_are_mutually_exclusive_in_both_builder_orders() {
    assert!(Cors::new()
        .allow_origin("*")
        .unwrap()
        .credentials(true)
        .is_err());
    assert!(Cors::new()
        .credentials(true)
        .unwrap()
        .allow_origin("*")
        .is_err());
    assert!(policy().allow_origin("*").is_err());
    assert!(Cors::new()
        .allow_origin("*")
        .unwrap()
        .allow_origin("https://client.example")
        .is_err());
    let app = application(policy().credentials(true).unwrap());
    for method in ["GET", "OPTIONS"] {
        let mut headers = vec![("origin", "https://client.example")];
        if method == "OPTIONS" {
            headers.push(("access-control-request-method", "GET"));
        }
        let response = app.respond(request(method, &headers));
        assert_eq!(
            response.headers().get("access-control-allow-credentials"),
            Some("true")
        );
        assert_eq!(response.headers().get("access-control-allow-origin"), Some("https://client.example"));
    }
    let app = application(Cors::new().allow_origin("*").unwrap());
    for origin in ["https://unlisted.example", "null"] {
        let response = app.respond(request("GET", &[("origin", origin)]));
        assert_eq!(response.status_code(), 200);
        assert_eq!(
            response.headers().get("access-control-allow-origin"),
            Some("*")
        );
        assert!(response.headers().get("access-control-allow-credentials").is_none());
    }
}

#[test]
fn invalid_configuration_is_rejected_before_requests() {
    for origin in [
        "",
        "null",
        "https://*.example.com",
        "https://example.com/",
        "https://user@example.com",
        "https://example.com?query",
        "https://example.com#fragment",
        "https://example.com:99999",
        "https://example.com:01",
        "https://bad_host",
        "https://-bad.test",
        "https://a..test",
        "https://é.test",
        "ftp://example.com",
        "https://example.com\r\nx: bad",
        "https://[::1",
        "https://[::1]/",
    ] {
        assert!(Cors::new().allow_origin(origin).is_err(), "{origin}");
    }
    for origin in [
        "https://example.com",
        "http://localhost:3000",
        "https://[::1]:8443",
        "http://127.0.0.1:3000",
    ] {
        assert!(Cors::new().allow_origin(origin).is_ok());
    }
    for method in ["", "*", "GET, POST", "GET\n", "TRACE", "connect"] {
        assert!(Cors::new().allow_methods([method]).is_err());
    }
    assert!(Cors::new().allow_methods([] as [&str; 0]).is_err());
    for header in ["", "*", "bad name", "x\r\ny"] {
        assert!(Cors::new().allow_headers([header]).is_err());
        assert!(Cors::new().expose_headers([header]).is_err());
    }
    assert!(Cors::new().expose_headers(["Set-Cookie"]).is_err());
    assert!(Cors::new().max_age(Duration::MAX).is_err());
    assert!(Cors::new().max_age(Duration::from_millis(1)).is_err());
    assert!(Cors::new()
        .allow_headers((0..129).map(|i| format!("x-{i}")))
        .is_err());
}

#[test]
fn cors_replaces_downstream_headers_and_preserves_vary_star() {
    let mut app = App::new();
    app.middleware(policy());
    app.route()
        .get("/", || {
            Response::text("ok")
                .header("access-control-allow-origin", "*")?
                .append_header("Access-Control-Allow-Origin", "https://attacker.test")?
                .header("access-control-allow-credentials", "true")?
                .header("vary", "*")
        })
        .unwrap();
    for headers in [vec![], vec![("origin", "https://client.example")]] {
        let response = app.respond(request("GET", &headers));
        assert_eq!(
            response
                .headers()
                .get_all("access-control-allow-origin")
                .count(),
            headers.len()
        );
        assert!(response.headers().get("access-control-allow-credentials").is_none());
        assert_eq!(response.headers().get("vary"), Some("*"));
    }
}

#[test]
fn default_security_headers_cover_success_and_public_error_responses() {
    for status in [200, 401, 403, 404, 500] {
        let mut app = App::new();
        app.middleware(SecurityHeaders::default());
        app.route()
            .get("/", move || -> Result<Response> {
                if status == 200 {
                    Ok(Response::text("ok"))
                } else {
                    Err(Error::rejected(status, "failure"))
                }
            })
            .unwrap();
        let response = app.respond(request("GET", &[]));
        assert_eq!(response.status_code(), status);
        assert_eq!(
            response.headers().get("x-content-type-options"),
            Some("nosniff")
        );
        assert_eq!(response.headers().get("x-frame-options"), Some("DENY"));
        assert_eq!(
            response.headers().get("referrer-policy"),
            Some("no-referrer")
        );
        for header in [
            "content-security-policy",
            "strict-transport-security",
            "permissions-policy",
        ] {
            assert!(response.headers().get(header).is_none());
        }
    }
}

#[test]
fn security_headers_are_configurable_and_override_handler_values() {
    let headers = SecurityHeaders::new()
        .frame_options("SAMEORIGIN")
        .unwrap()
        .referrer_policy("same-origin")
        .unwrap()
        .content_security_policy("default-src 'none'; frame-ancestors 'none'")
        .unwrap()
        .permissions_policy("camera=(), microphone=()")
        .unwrap()
        .hsts(Duration::from_secs(3600), true)
        .unwrap();
    let mut app = App::new();
    app.middleware(headers);
    app.route()
        .get("/", || {
            Response::text("ok")
                .header("x-frame-options", "DENY")?
                .append_header("X-Frame-Options", "invalid")?
                .header("x-custom", "preserved")
        })
        .unwrap();
    let response = app.respond(request("GET", &[]));
    assert_eq!(
        response.headers().get("x-frame-options"),
        Some("SAMEORIGIN")
    );
    assert_eq!(response.headers().get_all("x-frame-options").count(), 1);
    assert_eq!(
        response.headers().get("referrer-policy"),
        Some("same-origin")
    );
    assert_eq!(
        response.headers().get("content-security-policy"),
        Some("default-src 'none'; frame-ancestors 'none'")
    );
    assert_eq!(
        response.headers().get("permissions-policy"),
        Some("camera=(), microphone=()")
    );
    assert_eq!(
        response.headers().get("strict-transport-security"),
        Some("max-age=3600; includeSubDomains")
    );
    assert_eq!(response.headers().get("x-custom"), Some("preserved"));
    assert!(SecurityHeaders::new().frame_options("ALLOWALL").is_err());
    assert!(SecurityHeaders::new().referrer_policy("invalid").is_err());
    assert!(SecurityHeaders::new().hsts(Duration::MAX, false).is_err());
    for value in ["", "   ", "policy\r\nx-injected: yes", "policy\0"] {
        assert!(SecurityHeaders::new()
            .content_security_policy(value)
            .is_err());
        assert!(SecurityHeaders::new().permissions_policy(value).is_err());
    }
    let mut app = App::new();
    app.middleware(SecurityHeaders::new().hsts(Duration::ZERO, false).unwrap());
    assert_eq!(
        app.respond(request("GET", &[]))
            .headers()
            .get("strict-transport-security"),
        Some("max-age=0")
    );
}

#[cfg(feature = "auth")]
#[test]
fn security_layers_preserve_auth_challenges_and_preflight_skips_authentication() {
    use berserk::{
        auth::{MemoryTokenStore, Principal, TokenManager},
        Authenticated, RequireAbility,
    };
    use std::sync::Arc;
    let tokens = Arc::new(TokenManager::new(MemoryTokenStore::default()));
    let allowed = tokens
        .issue(Principal::new("user:1").unwrap(), ["read"], 0)
        .unwrap();
    let denied = tokens
        .issue(Principal::new("user:2").unwrap(), [] as [&str; 0], 0)
        .unwrap();
    let mut app = App::new();
    app.middleware(SecurityHeaders::default());
    app.middleware(policy());
    app.route()
        .middleware(Authenticated::new(tokens))
        .middleware(RequireAbility::new("read").unwrap())
        .get("/", |req: Request| {
            Response::text(req.user().unwrap().subject())
        })
        .unwrap();
    let preflight = app.respond(request(
        "OPTIONS",
        &[
            ("origin", "https://client.example"),
            ("access-control-request-method", "GET"),
            ("access-control-request-headers", "authorization"),
        ],
    ));
    assert_eq!(preflight.status_code(), 204);
    for (token, status) in [
        ("invalid", 401),
        (allowed.expose(), 200),
        (denied.expose(), 403),
    ] {
        let response = app.respond(request(
            "GET",
            &[
                ("origin", "https://client.example"),
                ("authorization", &format!("Bearer {token}")),
            ],
        ));
        assert_eq!(response.status_code(), status);
        assert_eq!(response.headers().get("access-control-allow-origin"), Some("https://client.example"));
        assert_eq!(response.headers().get("x-frame-options"), Some("DENY"));
        if status == 401 {
            assert_eq!(response.headers().get("www-authenticate"), Some("Bearer"));
        }
    }
}
