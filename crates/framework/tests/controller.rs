use framework::{App, Headers, Method, Request, Response};

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
}

#[test]
fn controller_action_can_receive_typed_route_parameter() {
    let mut app = App::new();
    app.get_param("/users/{id}", "id", UserController::show)
        .unwrap();

    assert_eq!(app.handle(request("/users/42")).unwrap().body(), b"42");
    assert_eq!(
        app.handle(request("/users/not-a-number"))
            .unwrap()
            .status_code(),
        400
    );
}

#[test]
fn request_can_parse_route_parameters_explicitly() {
    let mut app = App::new();
    app.get("/users/{id}", |request| {
        let id = request.param_as::<u64>("id").unwrap().unwrap();
        Response::text(id.to_string())
    })
    .unwrap();

    assert_eq!(app.handle(request("/users/7")).unwrap().body(), b"7");
}
