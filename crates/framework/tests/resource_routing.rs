use framework::{
    ApiResourceController, App, Error, Headers, Method, Request, ResourceController, Response,
    Result, RouteError,
};

fn request(method: &str, path: &str) -> Request {
    Request::new(
        Method::new(method).unwrap(),
        path,
        Headers::new(),
        Vec::new(),
    )
    .unwrap()
}

struct Users;

impl ApiResourceController for Users {
    type Id = u64;

    fn index(&self, _request: Request) -> Result<Response> {
        Ok(Response::text("index"))
    }

    fn store(&self, _request: Request) -> Result<Response> {
        Ok(Response::text("store"))
    }

    fn show(&self, id: Self::Id, _request: Request) -> Result<Response> {
        Ok(Response::text(format!("show:{id}")))
    }

    fn update(&self, id: Self::Id, request: Request) -> Result<Response> {
        Ok(Response::text(format!("update:{}:{id}", request.method())))
    }

    fn destroy(&self, id: Self::Id, _request: Request) -> Result<Response> {
        Ok(Response::text(format!("destroy:{id}")))
    }
}

impl ResourceController for Users {
    fn create(&self, _request: Request) -> Result<Response> {
        Ok(Response::text("create"))
    }

    fn edit(&self, id: Self::Id, _request: Request) -> Result<Response> {
        Ok(Response::text(format!("edit:{id}")))
    }
}

#[test]
fn named_routes_generate_paths_and_reject_conflicts() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route
            .name("users.show")
            .get("/users/{id}", |id: u64| Response::text(id.to_string()))
            .unwrap();
        route
            .name("files.show")
            .get("/files/{name}", |name: String| Response::text(name))
            .unwrap();
    }

    assert_eq!(
        app.path_for("users.show", &[("id", "42")]).unwrap(),
        "/users/42"
    );
    assert_eq!(
        app.path_for("files.show", &[("name", "a/b c")]).unwrap(),
        "/files/a%2Fb%20c"
    );

    let missing = app.path_for("users.show", &[]).unwrap_err();
    assert!(matches!(
        missing,
        Error::Routing(RouteError::MissingRouteParameter(ref name)) if name == "id"
    ));

    let unknown = app
        .path_for("users.show", &[("id", "42"), ("extra", "x")])
        .unwrap_err();
    assert!(matches!(
        unknown,
        Error::Routing(RouteError::UnknownRouteParameter(ref name)) if name == "extra"
    ));

    let duplicate = {
        let mut route = app.route();
        route
            .name("users.show")
            .get("/accounts/{id}", |id: u64| Response::text(id.to_string()))
    }
    .unwrap_err();
    assert!(matches!(
        duplicate,
        Error::Routing(RouteError::DuplicateRouteName(ref name)) if name == "users.show"
    ));

    assert_eq!(
        app.handle(request("GET", "/accounts/1"))
            .unwrap()
            .status_code(),
        404
    );
}

#[test]
fn resource_registers_full_rest_surface_and_names() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route.resource("/users", Users).unwrap();
    }

    for (name, params, expected) in [
        ("users.index", &[][..], "/users"),
        ("users.create", &[][..], "/users/create"),
        ("users.store", &[][..], "/users"),
        ("users.show", &[("id", "7")][..], "/users/7"),
        ("users.edit", &[("id", "7")][..], "/users/7/edit"),
        ("users.update", &[("id", "7")][..], "/users/7"),
        ("users.destroy", &[("id", "7")][..], "/users/7"),
    ] {
        assert_eq!(app.path_for(name, params).unwrap(), expected);
    }

    for (method, path, expected) in [
        ("GET", "/users", "index"),
        ("GET", "/users/create", "create"),
        ("POST", "/users", "store"),
        ("GET", "/users/3", "show:3"),
        ("GET", "/users/3/edit", "edit:3"),
        ("PUT", "/users/3", "update:PUT:3"),
        ("PATCH", "/users/3", "update:PATCH:3"),
        ("DELETE", "/users/3", "destroy:3"),
    ] {
        assert_eq!(
            app.handle(request(method, path)).unwrap().body(),
            expected.as_bytes(),
            "{method} {path}"
        );
    }
}

#[test]
fn api_resource_omits_web_routes_and_derives_prefix_names() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route.prefix("/api").api_resource("/users", Users).unwrap();
    }

    assert_eq!(
        app.path_for("api.users.index", &[]).unwrap(),
        "/api/users"
    );
    assert_eq!(
        app.path_for("api.users.show", &[("id", "9")]).unwrap(),
        "/api/users/9"
    );
    assert!(matches!(
        app.path_for("api.users.create", &[]),
        Err(Error::Routing(RouteError::UnknownRouteName(ref name))) if name == "api.users.create"
    ));
    assert!(matches!(
        app.path_for("api.users.edit", &[("id", "9")]),
        Err(Error::Routing(RouteError::UnknownRouteName(ref name))) if name == "api.users.edit"
    ));

    assert_eq!(
        app.handle(request("GET", "/api/users/9")).unwrap().body(),
        b"show:9"
    );
}

#[test]
fn resource_registration_is_atomic_on_collision() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route
            .get("/users/{id}", |id: u64| Response::text(format!("existing:{id}")))
            .unwrap();
    }

    let error = {
        let mut route = app.route();
        route.resource("/users", Users)
    }
    .unwrap_err();
    assert!(matches!(error, Error::Routing(RouteError::DuplicateRoute)));

    assert_eq!(
        app.handle(request("GET", "/users"))
            .unwrap()
            .status_code(),
        404
    );
    assert_eq!(
        app.handle(request("GET", "/users/4")).unwrap().body(),
        b"existing:4"
    );
}

#[test]
fn resources_and_fallback_coexist() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route.api_resource("/users", Users).unwrap();
        route
            .fallback(|request: Request| {
                Response::text(format!("missing:{}", request.path())).status(404)
            })
            .unwrap();
    }

    assert_eq!(
        app.handle(request("GET", "/users/5")).unwrap().body(),
        b"show:5"
    );
    assert_eq!(
        app.handle(request("GET", "/missing")).unwrap().body(),
        b"missing:/missing"
    );
    assert_eq!(
        app.handle(request("POST", "/users/5"))
            .unwrap()
            .status_code(),
        405
    );
}
