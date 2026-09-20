use berserk::{App, Error, Headers, Method, Request, Response, RouteError};

fn request(path: &str) -> Request {
    Request::new(
        Method::new("GET").unwrap(),
        path,
        Headers::new(),
        Vec::new(),
    )
    .unwrap()
}

struct UserController;

impl UserController {
    fn show(id: u64) -> Response {
        Response::text(id.to_string())
    }

    fn post(user_id: u64, post_id: u32) -> Response {
        Response::text(format!("{user_id}:{post_id}"))
    }

    fn post_with_request(user_id: u64, post_id: u32, request: Request) -> Response {
        Response::text(format!("{}:{user_id}:{post_id}", request.path()))
    }
}

#[test]
fn controller_action_can_receive_typed_route_parameter() {
    let mut app = App::new();
    app.route().get("/users/{id}", UserController::show).unwrap();

    assert_eq!(app.handle(request("/users/42")).unwrap().body(), b"42");
    assert_eq!(
        app.handle(request("/users/not-a-number"))
            .unwrap()
            .status_code(),
        400
    );
}

#[test]
fn controller_action_receives_multiple_parameters_in_route_order() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route
            .get("/users/{user_id}/posts/{post_id}", UserController::post)
            .unwrap();
        route
            .get(
                "/accounts/{user_id}/posts/{post_id}",
                UserController::post_with_request,
            )
            .unwrap();
    }

    assert_eq!(
        app.handle(request("/users/42/posts/7")).unwrap().body(),
        b"42:7"
    );
    assert_eq!(
        app.handle(request("/accounts/9/posts/3")).unwrap().body(),
        b"/accounts/9/posts/3:9:3"
    );

    assert_eq!(
        app.handle(request("/users/not-a-number/posts/7"))
            .unwrap()
            .status_code(),
        400
    );
    assert_eq!(
        app.handle(request("/users/42/posts/not-a-number"))
            .unwrap()
            .status_code(),
        400
    );
}

#[test]
fn multi_parameter_handler_registration_checks_route_shape() {
    let mut app = App::new();
    let error = {
        let mut route = app.route();
        route.get("/users/{id}", UserController::post)
    }
    .unwrap_err();

    assert!(matches!(
        error,
        Error::Routing(RouteError::ParameterCountMismatch {
            expected: 2,
            actual: 1
        })
    ));
}

#[test]
fn request_can_parse_route_parameters_explicitly() {
    let mut app = App::new();
    app.route().get("/users/{id}", |request: Request| {
        let id = request.param_as::<u64>("id").unwrap().unwrap();
        Response::text(id.to_string())
    })
    .unwrap();

    assert_eq!(app.handle(request("/users/7")).unwrap().body(), b"7");
}
