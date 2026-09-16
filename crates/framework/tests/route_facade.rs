use framework::{App, Headers, Method, Middleware, Next, Request, Response, Result, RouteError};

fn request(method: &str, path: &str) -> Request {
    Request::new(
        Method::new(method).unwrap(),
        path,
        Headers::new(),
        Vec::new(),
    )
    .unwrap()
}

fn show(id: u64) -> Response {
    Response::text(id.to_string())
}

fn update(id: u64, _request: Request) -> Response {
    Response::text(format!("updated:{id}"))
}

struct RouteMarker;

impl Middleware for RouteMarker {
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {
        next.run(request)?.header("x-route-middleware", "yes")
    }
}

#[test]
fn route_instance_registers_standard_verbs_and_typed_handlers() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route.get("/users/{id}", show).unwrap();
        route.post("/users", || Response::text("created")).unwrap();
        route.put("/users/{id}", update).unwrap();
        route.patch("/users/{id}", update).unwrap();
        route.delete("/users/{id}", show).unwrap();
    }

    assert_eq!(app.handle(request("GET", "/users/7")).unwrap().body(), b"7");
    assert_eq!(
        app.handle(request("POST", "/users")).unwrap().body(),
        b"created"
    );
    assert_eq!(
        app.handle(request("PUT", "/users/8")).unwrap().body(),
        b"updated:8"
    );
    assert_eq!(
        app.handle(request("PATCH", "/users/9")).unwrap().body(),
        b"updated:9"
    );
    assert_eq!(
        app.handle(request("DELETE", "/users/10")).unwrap().body(),
        b"10"
    );
}

#[test]
fn prefix_and_middleware_are_scoped_and_composable() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route.get("/public", || Response::text("public")).unwrap();

        route
            .prefix("/api")
            .middleware(RouteMarker)
            .group(|route| {
                route.get("/users/{id}", show)?;
                route
                    .prefix("/v1")
                    .get("/status", || Response::text("ok"))?;
                Ok(())
            })
            .unwrap();
    }

    let response = app.handle(request("GET", "/api/users/3")).unwrap();
    assert_eq!(response.body(), b"3");
    assert_eq!(response.headers().get("x-route-middleware"), Some("yes"));

    let nested = app.handle(request("GET", "/api/v1/status")).unwrap();
    assert_eq!(nested.body(), b"ok");
    assert_eq!(nested.headers().get("x-route-middleware"), Some("yes"));

    let public = app.handle(request("GET", "/public")).unwrap();
    assert_eq!(public.body(), b"public");
    assert_eq!(public.headers().get("x-route-middleware"), None);

    assert_eq!(
        app.handle(request("GET", "/users/3"))
            .unwrap()
            .status_code(),
        404
    );
}

#[test]
fn failed_route_group_is_atomic() {
    let mut app = App::new();
    let error = {
        let mut route = app.route();
        route.prefix("/api").group(|route| {
            route.get("/users/{id}", show)?;
            route.get("/users/{other}", show)?;
            Ok(())
        })
    }
    .unwrap_err();

    assert!(matches!(
        error,
        framework::Error::Routing(RouteError::DuplicateRoute)
    ));
    assert_eq!(
        app.handle(request("GET", "/api/users/1"))
            .unwrap()
            .status_code(),
        404
    );
}

#[test]
fn fallback_runs_only_when_no_pattern_matches() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route.get("/users", || Response::text("users")).unwrap();
        route
            .fallback(|request: Request| {
                Response::text(format!("missing:{}", request.path())).status(404)
            })
            .unwrap();
    }

    assert_eq!(
        app.handle(request("GET", "/missing")).unwrap().body(),
        b"missing:/missing"
    );
    assert_eq!(
        app.handle(request("POST", "/users"))
            .unwrap()
            .status_code(),
        405
    );
}

#[test]
fn fallback_supports_middleware_but_rejects_invalid_registration() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route
            .middleware(RouteMarker)
            .fallback(|| Response::text("missing").status(404))
            .unwrap();
    }

    let response = app.handle(request("GET", "/missing")).unwrap();
    assert_eq!(response.status_code(), 404);
    assert_eq!(response.headers().get("x-route-middleware"), Some("yes"));

    let duplicate = {
        let mut route = app.route();
        route.fallback(Response::empty)
    }
    .unwrap_err();
    assert!(matches!(
        duplicate,
        framework::Error::Routing(RouteError::DuplicateFallback)
    ));

    let mut prefixed_app = App::new();
    let scoped = {
        let mut route = prefixed_app.route();
        route
            .prefix("/api")
            .fallback(|| Response::text("missing"))
    }
    .unwrap_err();
    assert!(matches!(
        scoped,
        framework::Error::Routing(RouteError::ScopedFallback)
    ));

    let mut typed_app = App::new();
    let typed = {
        let mut route = typed_app.route();
        route.fallback(show)
    }
    .unwrap_err();
    assert!(matches!(
        typed,
        framework::Error::Routing(RouteError::ParameterCountMismatch {
            expected: 1,
            actual: 0
        })
    ));
}
