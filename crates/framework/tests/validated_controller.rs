use framework::{
    App, Error, FromJson, Headers, Json, Method, Request, Response, ValidateInput, Validated,
    ValidationErrors,
};
use framework_validation::sanitize;

#[derive(Debug, PartialEq, Eq)]
struct UserInput {
    name: String,
}

impl FromJson for UserInput {
    fn from_json(value: &Json) -> std::result::Result<Self, ValidationErrors> {
        let mut errors = ValidationErrors::default();
        let name = match value.get("name").and_then(Json::as_str) {
            Some(name) => name.to_owned(),
            None => {
                errors.add("name", "required", "The name field is required.");
                String::new()
            }
        };
        errors.finish()?;
        Ok(Self { name })
    }
}

impl ValidateInput for UserInput {
    fn sanitize(&mut self) {
        sanitize::trim(&mut self.name);
    }

    fn validate(&self) -> std::result::Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();
        errors.min_length("name", &self.name, 2);
        errors.finish()
    }
}

fn request(method: &str, path: &str, body: &str, content_type: bool) -> Request {
    let mut headers = Headers::new();
    if content_type {
        headers.insert("content-type", "application/json").unwrap();
    }
    Request::new(
        Method::new(method).unwrap(),
        path,
        headers,
        body.as_bytes().to_vec(),
    )
    .unwrap()
}

fn store(input: Validated<UserInput>) -> Response {
    Response::text(input.name.clone())
}

fn update(id: u64, input: Validated<UserInput>) -> Response {
    Response::text(format!("{id}:{}", input.name))
}

fn store_with_request(input: Validated<UserInput>, request: Request) -> Response {
    Response::text(format!("{}:{}", request.method().as_str(), input.name))
}

fn update_with_request(id: u64, input: Validated<UserInput>, request: Request) -> Response {
    Response::text(format!("{id}:{}:{}", input.name, request.path()))
}

fn input_status(error: Error) -> u16 {
    match error {
        Error::Input(error) => error.response().status_code(),
        other => panic!("expected input error, got {other}"),
    }
}

#[test]
fn validated_controller_input_is_sanitized_before_validation() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route.post("/users", store).unwrap();
        route.put("/users/{id}", update).unwrap();
    }

    let stored = app
        .handle(request("POST", "/users", r#"{"name":"  Ada  "}"#, true))
        .unwrap();
    assert_eq!(stored.body(), b"Ada");

    let updated = app
        .handle(request("PUT", "/users/7", r#"{"name":"  Grace  "}"#, true))
        .unwrap();
    assert_eq!(updated.body(), b"7:Grace");
}

#[test]
fn validated_controller_can_also_receive_request_context() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route.post("/context", store_with_request).unwrap();
        route
            .patch("/users/{id}/context", update_with_request)
            .unwrap();
    }

    let stored = app
        .handle(request("POST", "/context", r#"{"name":"  Ada  "}"#, true))
        .unwrap();
    assert_eq!(stored.body(), b"POST:Ada");

    let updated = app
        .handle(request(
            "PATCH",
            "/users/12/context",
            r#"{"name":"  Grace  "}"#,
            true,
        ))
        .unwrap();
    assert_eq!(updated.body(), b"12:Grace:/users/12/context");
}

#[test]
fn validated_controller_input_reuses_existing_input_errors() {
    let mut app = App::new();
    {
        let mut route = app.route();
        route.post("/users", store).unwrap();
        route.put("/users/{id}", update).unwrap();
    }

    let validation = app
        .handle(request("POST", "/users", r#"{"name":" A "}"#, true))
        .unwrap_err();
    assert_eq!(input_status(validation), 422);

    let content_type = app
        .handle(request("POST", "/users", r#"{"name":"Ada"}"#, false))
        .unwrap_err();
    assert_eq!(input_status(content_type), 415);

    let invalid_id = app
        .handle(request("PUT", "/users/nope", "not-json", false))
        .unwrap();
    assert_eq!(invalid_id.status_code(), 400);
}
