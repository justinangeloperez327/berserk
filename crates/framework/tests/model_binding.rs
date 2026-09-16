#![cfg(feature = "claw")]

use framework::{
    claw::{field, Model, Row, Value},
    database::{
        Capabilities, Connection, Database, DatabaseError, Driver, ErrorKind, Execution, Statement,
        Transaction, TransactionOptions,
    },
    App, Error, Headers, Method, Request, Response,
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
        if statement.bindings() != [Value::U64(7)] {
            return Ok(Vec::new());
        }
        Ok(vec![Row::new(vec![
            framework::database::Column::new("id", 7_u64),
            framework::database::Column::new("name", "Ada"),
        ])?])
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

fn show(user: User) -> Response {
    Response::text(format!("{}:{}", user.id, user.name))
}

fn show_with_request(user: User, request: Request) -> Response {
    Response::text(format!("{}:{}", user.name, request.path()))
}

#[test]
fn controller_can_receive_a_bound_claw_model() {
    let mut app = App::new();
    app.state(Database::new(|| Ok(FakeConnection))).unwrap();
    {
        let mut route = app.route();
        route.get("/users/{user}", show).unwrap();
        route
            .get("/accounts/{user}", show_with_request)
            .unwrap();
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
    assert_eq!(
        app.handle(request("/users/99")).unwrap().status_code(),
        404
    );
}

#[test]
fn model_binding_requires_database_state() {
    let mut app = App::new();
    app.route().get("/users/{user}", show).unwrap();

    let error = app.handle(request("/users/7")).unwrap_err();
    assert!(matches!(error, Error::Configuration(_)));
}
