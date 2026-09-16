#![cfg(feature = "claw")]

use framework::{
    claw::{field, Model, Row, Value},
    database::{
        Capabilities, Connection, Database, DatabaseError, Driver, ErrorKind, Execution, Statement,
        Transaction, TransactionOptions,
    },
    App, Error, FromJson, Headers, Json, Method, Request, Response, ValidateInput, Validated,
    ValidationErrors,
};
use framework_validation::sanitize;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Debug)]
struct User {
    id: u64,
    name: String,
}

impl Model for User {
    const TABLE: &'static str = "users";

    fn from_row(row: &Row) -> framework::claw::Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            name: field(row, "name")?,
        })
    }

    fn key(&self) -> Value {
        self.id.into()
    }
}

#[derive(Debug)]
struct Post {
    id: u64,
    title: String,
}

impl Model for Post {
    const TABLE: &'static str = "posts";

    fn from_row(row: &Row) -> framework::claw::Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            title: field(row, "title")?,
        })
    }

    fn key(&self) -> Value {
        self.id.into()
    }
}

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

struct FakeConnection;

impl Connection for FakeConnection {
    fn driver(&self) -> Driver {
        Driver::Sqlite
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
    }

    fn execute(&mut self, _statement: &Statement) -> framework::database::Result<Execution> {
        Ok(Execution {
            affected_rows: 0,
            last_insert_id: None,
        })
    }

    fn query(&mut self, statement: &Statement) -> framework::database::Result<Vec<Row>> {
        if statement.sql().contains("\"users\"") && statement.bindings() == [Value::U64(7)] {
            return Ok(vec![Row::new(vec![
                framework::database::Column::new("id", 7_u64),
                framework::database::Column::new("name", "Ada"),
            ])?]);
        }
        if statement.sql().contains("\"posts\"") && statement.bindings() == [Value::U64(3)] {
            return Ok(vec![Row::new(vec![
                framework::database::Column::new("id", 3_u64),
                framework::database::Column::new("title", "First"),
            ])?]);
        }
        Ok(Vec::new())
    }

    fn begin(
        &mut self,
        _options: TransactionOptions,
    ) -> framework::database::Result<Box<dyn Transaction + '_>> {
        Err(DatabaseError::new(
            ErrorKind::Transaction,
            "transactions are not used by this test",
        ))
    }

    fn ping(&mut self) -> framework::database::Result<()> {
        Ok(())
    }
}

fn request(path: &str) -> Request {
    Request::new(
        Method::new("GET").unwrap(),
        path,
        Headers::new(),
        Vec::new(),
    )
    .unwrap()
}

fn body_request(method: &str, path: &str, body: &str, content_type: bool) -> Request {
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

fn input_status(error: Error) -> u16 {
    match error {
        Error::Input(error) => error.response().status_code(),
        other => panic!("expected input error, got {other}"),
    }
}

fn show(user: User) -> Response {
    Response::text(format!("{}:{}", user.id, user.name))
}

fn show_with_request(user: User, request: Request) -> Response {
    Response::text(format!("{}:{}", user.name, request.path()))
}

fn show_nested(user: User, post: Post) -> Response {
    Response::text(format!("{}:{}:{}", user.id, post.id, post.title))
}

fn show_nested_with_request(user: User, post: Post, request: Request) -> Response {
    Response::text(format!(
        "{}:{}:{}:{}",
        user.id,
        post.id,
        post.title,
        request.path()
    ))
}

fn update(user: User, input: Validated<UserInput>) -> Response {
    Response::text(format!("{}:{}", user.id, input.name))
}

fn update_with_request(user: User, input: Validated<UserInput>, request: Request) -> Response {
    Response::text(format!("{}:{}:{}", user.id, input.name, request.path()))
}

#[test]
fn controller_can_receive_a_bound_claw_model() {
    let mut app = App::new();
    app.state(Database::new(|| Ok(FakeConnection))).unwrap();
    {
        let mut route = app.route();
        route.get("/users/{user}", show).unwrap();
        route.get("/accounts/{user}", show_with_request).unwrap();
    }

    assert_eq!(app.handle(request("/users/7")).unwrap().body(), b"7:Ada");
    assert_eq!(
        app.handle(request("/accounts/7")).unwrap().body(),
        b"Ada:/accounts/7"
    );
    assert_eq!(
        app.handle(request("/users/not-a-number"))
            .unwrap()
            .status_code(),
        400
    );
    assert_eq!(app.handle(request("/users/99")).unwrap().status_code(), 404);
}

#[test]
fn controller_can_receive_two_bound_claw_models() {
    let acquisitions = Arc::new(AtomicUsize::new(0));
    let factory_acquisitions = Arc::clone(&acquisitions);
    let mut app = App::new();
    app.state(Database::new(move || {
        factory_acquisitions.fetch_add(1, Ordering::SeqCst);
        Ok(FakeConnection)
    }))
    .unwrap();
    {
        let mut route = app.route();
        route
            .get("/users/{user}/posts/{post}", show_nested)
            .unwrap();
        route
            .get(
                "/users/{user}/posts/{post}/context",
                show_nested_with_request,
            )
            .unwrap();
    }

    let response = app.handle(request("/users/7/posts/3")).unwrap();
    assert_eq!(response.body(), b"7:3:First");
    assert_eq!(acquisitions.load(Ordering::SeqCst), 1);

    let contextual = app
        .handle(request("/users/7/posts/3/context"))
        .unwrap();
    assert_eq!(contextual.body(), b"7:3:First:/users/7/posts/3/context");
    assert_eq!(acquisitions.load(Ordering::SeqCst), 2);

    assert_eq!(
        app.handle(request("/users/7/posts/not-a-number"))
            .unwrap()
            .status_code(),
        400
    );
    assert_eq!(
        app.handle(request("/users/99/posts/3"))
            .unwrap()
            .status_code(),
        404
    );
    assert_eq!(
        app.handle(request("/users/7/posts/99"))
            .unwrap()
            .status_code(),
        404
    );
}

#[test]
fn bound_model_can_precede_validated_input() {
    let mut app = App::new();
    app.state(Database::new(|| Ok(FakeConnection))).unwrap();
    {
        let mut route = app.route();
        route.put("/users/{user}", update).unwrap();
        route
            .patch("/users/{user}/context", update_with_request)
            .unwrap();
    }

    let updated = app
        .handle(body_request(
            "PUT",
            "/users/7",
            r#"{"name":"  Grace  "}"#,
            true,
        ))
        .unwrap();
    assert_eq!(updated.body(), b"7:Grace");

    let contextual = app
        .handle(body_request(
            "PATCH",
            "/users/7/context",
            r#"{"name":"  Linus  "}"#,
            true,
        ))
        .unwrap();
    assert_eq!(contextual.body(), b"7:Linus:/users/7/context");
}

#[test]
fn model_binding_finishes_before_body_validation() {
    let mut app = App::new();
    app.state(Database::new(|| Ok(FakeConnection))).unwrap();
    app.route().put("/users/{user}", update).unwrap();

    let invalid_key = app
        .handle(body_request(
            "PUT",
            "/users/not-a-number",
            "not-json",
            false,
        ))
        .unwrap();
    assert_eq!(invalid_key.status_code(), 400);

    let missing_model = app
        .handle(body_request("PUT", "/users/99", "not-json", false))
        .unwrap();
    assert_eq!(missing_model.status_code(), 404);

    let missing_content_type = app
        .handle(body_request(
            "PUT",
            "/users/7",
            r#"{"name":"Grace"}"#,
            false,
        ))
        .unwrap_err();
    assert_eq!(input_status(missing_content_type), 415);

    let invalid_input = app
        .handle(body_request("PUT", "/users/7", r#"{"name":" A "}"#, true))
        .unwrap_err();
    assert_eq!(input_status(invalid_input), 422);
}

#[test]
fn model_binding_requires_database_state() {
    let mut app = App::new();
    app.route().get("/users/{user}", show).unwrap();

    let error = app.handle(request("/users/7")).unwrap_err();
    assert!(matches!(error, Error::Configuration(_)));
}
