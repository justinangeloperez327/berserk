#![cfg(feature = "claw")]

use framework::{
    claw::{field, Model, PersistableModel, Row, Value},
    database::{
        Capabilities, Connection, Database, DatabaseError, Driver, ErrorKind, Execution, Statement,
        Transaction, TransactionOptions,
    },
    App, FromJson, Headers, Json, Method, Request, Response, Result, ValidateInput, Validated,
    ValidationErrors,
};
use framework_validation::sanitize;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
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

impl PersistableModel for User {
    fn values_for_save(&self) -> Vec<(&'static str, Value)> {
        vec![("name", self.name.clone().into())]
    }
}

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

struct FakeConnection {
    executed: Arc<Mutex<Vec<Statement>>>,
}

impl Connection for FakeConnection {
    fn driver(&self) -> Driver {
        Driver::Sqlite
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
    }

    fn execute(&mut self, statement: &Statement) -> framework::database::Result<Execution> {
        self.executed.lock().unwrap().push(statement.clone());
        Ok(Execution {
            affected_rows: 1,
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

fn update(mut user: User, input: Validated<UserInput>, request: Request) -> Result<Response> {
    user.name = input.name.clone();
    let mut connection = request.connection()?;
    user.save(&mut *connection)?;
    Ok(Response::empty().status(204))
}

#[test]
fn bound_model_validated_input_and_save_share_one_connection() {
    let acquisitions = Arc::new(AtomicUsize::new(0));
    let executed = Arc::new(Mutex::new(Vec::new()));
    let factory_acquisitions = Arc::clone(&acquisitions);
    let factory_executed = Arc::clone(&executed);
    let mut app = App::new();
    app.database(Database::new(move || {
        factory_acquisitions.fetch_add(1, Ordering::SeqCst);
        Ok(FakeConnection {
            executed: Arc::clone(&factory_executed),
        })
    }))
    .unwrap();
    app.route().put("/users/{user}", update).unwrap();

    let mut headers = Headers::new();
    headers.insert("content-type", "application/json").unwrap();
    let request = Request::new(
        Method::new("PUT").unwrap(),
        "/users/7",
        headers,
        br#"{"name":"  Grace  "}"#.to_vec(),
    )
    .unwrap();

    let response = app.handle(request).unwrap();

    assert_eq!(response.status_code(), 204);
    assert_eq!(acquisitions.load(Ordering::SeqCst), 1);
    let executed = executed.lock().unwrap();
    assert_eq!(executed.len(), 1);
    assert_eq!(
        executed[0].sql(),
        "UPDATE \"users\" SET \"name\" = ? WHERE \"id\" = ?"
    );
    assert_eq!(
        executed[0].bindings(),
        &[Value::Text("Grace".into()), Value::U64(7)]
    );
}
