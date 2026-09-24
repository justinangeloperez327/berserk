use berserk::{
    App, FromJson, Headers, Json, Method, Request, Response, ValidateInput, Validated,
    ValidationErrors,
};
use berserk_validation::sanitize;

#[derive(Debug)]
struct Input {
    name: String,
}
impl FromJson for Input {
    fn from_json(value: &Json) -> std::result::Result<Self, ValidationErrors> {
        let mut errors = ValidationErrors::default();
        let name = value
            .get("name")
            .and_then(Json::as_str)
            .unwrap_or_default()
            .to_owned();
        if name.is_empty() {
            errors.add("name", "required", "The name field is required.");
        }
        errors.finish()?;
        Ok(Self { name })
    }
}
impl ValidateInput for Input {
    fn sanitize(&mut self) {
        sanitize::trim(&mut self.name);
    }
    fn validate(&self) -> std::result::Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();
        errors.min_length("name", &self.name, 2);
        errors.finish()
    }
}

fn request(method: &str, path: &str, body: &[u8], json: bool) -> Request {
    let mut headers = Headers::new();
    if json {
        headers.insert("content-type", "application/json").unwrap();
    }
    Request::new(Method::new(method).unwrap(), path, headers, body.to_vec()).unwrap()
}

#[test]
fn lifecycle_status_contract_is_consistent() {
    let mut app = App::new();
    app.route()
        .get("/users/{id}", |id: u64| Response::text(id.to_string()))
        .unwrap();
    app.route()
        .post("/users", |input: Validated<Input>| {
            Response::text(input.name.clone()).status(201)
        })
        .unwrap();
    app.route()
        .delete("/users/{id}", |_id: u64| Response::empty().status(204))
        .unwrap();

    assert_eq!(
        app.respond(request("GET", "/missing", b"", false))
            .status_code(),
        404
    );
    let wrong = app.respond(request("POST", "/users/7", b"", false));
    assert_eq!(wrong.status_code(), 405);
    assert_eq!(wrong.headers().get("allow"), Some("GET, HEAD"));
    assert_eq!(
        app.respond(request("GET", "/users/nope", b"", false))
            .status_code(),
        400
    );
    assert_eq!(
        app.respond(request("POST", "/users", br#"{"name":" A "}"#, true))
            .status_code(),
        422
    );
    assert_eq!(
        app.respond(request("POST", "/users", br#"{"name":" Ada "}"#, true))
            .status_code(),
        201
    );
    assert_eq!(
        app.respond(request("DELETE", "/users/7", b"", false))
            .status_code(),
        204
    );
}

#[test]
fn malformed_json_and_content_type_are_public_client_errors() {
    let mut app = App::new();
    app.route()
        .post("/users", |_input: Validated<Input>| Response::empty())
        .unwrap();

    assert_eq!(
        app.respond(request("POST", "/users", br#"{"name":"Ada""#, true))
            .status_code(),
        400
    );
    assert_eq!(
        app.respond(request("POST", "/users", br#"{"name":"Ada"}"#, false))
            .status_code(),
        415
    );
}

#[test]
fn head_uses_get_metadata_and_never_exposes_body() {
    let mut app = App::new();
    app.route()
        .get("/resource", || Response::text("representation"))
        .unwrap();
    let response = app.respond(request("HEAD", "/resource", b"", false));
    assert_eq!(response.status_code(), 200);
    assert!(response.body().is_empty());
    assert_eq!(response.representation_length(), "representation".len());
}
