#![cfg(all(feature = "sqlite", feature = "claw"))]

use berserk::{
    claw::{field, Model, Row, Value},
    database::{drivers::sqlite::SqliteConnection, Database, Query},
    App, Headers, Method, Request, Response,
};

#[derive(Debug)] struct User { id: i64, name: String }
impl Model for User {
    const TABLE: &'static str = "users";
    fn from_row(row: &Row) -> berserk::claw::Result<Self> { Ok(Self { id: field(row, "id")?, name: field(row, "name")? }) }
    fn key(&self) -> Value { self.id.into() }
}

fn database() -> Database {
    Database::new(|| {
        let mut c = SqliteConnection::in_memory()?;
        Query::raw("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").execute(&mut c)?;
        Query::table("users").insert([("id", Value::I64(7)), ("name", Value::from("Ada"))]).execute(&mut c)?;
        Ok(c)
    })
}
fn request(path: &str) -> Request {
    Request::new(Method::new("GET").unwrap(), path, Headers::new(), Vec::new()).unwrap()
}
fn show(user: User) -> Response { Response::text(format!("{}:{}", user.id, user.name)) }

#[test]
fn request_route_model_binding_claw_database_response_is_one_lifecycle() {
    let mut app = App::new();
    app.database(database()).unwrap();
    app.route().get("/users/{user}", show).unwrap();

    let found = app.respond(request("/users/7"));
    assert_eq!(found.status_code(), 200);
    assert_eq!(found.body(), b"7:Ada");

    assert_eq!(app.respond(request("/users/nope")).status_code(), 400);
    assert_eq!(app.respond(request("/users/99")).status_code(), 404);
}
