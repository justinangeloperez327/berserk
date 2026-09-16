use claw_orm::{field, Model, Result, Row, Statement, Value};
use framework_database::{
    Capabilities, Column, Connection, DatabaseError, Driver, ErrorKind, Execution, Transaction,
    TransactionOptions,
};

#[derive(Debug, PartialEq)]
struct User {
    id: u64,
    name: String,
}

impl Model for User {
    const TABLE: &'static str = "users";

    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            name: field(row, "name")?,
        })
    }

    fn key(&self) -> Value {
        self.id.into()
    }
}

#[derive(Default)]
struct FakeConnection {
    rows: Vec<Row>,
    statements: Vec<Statement>,
}

impl FakeConnection {
    fn with_rows(rows: Vec<Row>) -> Self {
        Self {
            rows,
            ..Self::default()
        }
    }
}

impl Connection for FakeConnection {
    fn driver(&self) -> Driver {
        Driver::Sqlite
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
    }

    fn execute(&mut self, _statement: &Statement) -> Result<Execution> {
        Ok(Execution {
            affected_rows: 0,
            last_insert_id: None,
        })
    }

    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        self.statements.push(statement.clone());
        Ok(std::mem::take(&mut self.rows))
    }

    fn begin(&mut self, _options: TransactionOptions) -> Result<Box<dyn Transaction + '_>> {
        Err(DatabaseError::new(
            ErrorKind::Transaction,
            "transactions are not used by this test",
        ))
    }

    fn ping(&mut self) -> Result<()> {
        Ok(())
    }
}

fn user_row(id: u64, name: &str) -> Row {
    Row::new(vec![Column::new("id", id), Column::new("name", name)]).unwrap()
}

#[test]
fn fresh_returns_current_database_state_without_mutating_original() {
    let user = User {
        id: 7,
        name: "Ada".into(),
    };
    let mut connection = FakeConnection::with_rows(vec![user_row(7, "Grace")]);

    let fresh = user.fresh(&mut connection).unwrap().unwrap();

    assert_eq!(user.name, "Ada");
    assert_eq!(fresh.name, "Grace");
    assert_eq!(connection.statements.len(), 1);
    assert_eq!(connection.statements[0].bindings(), &[Value::U64(7)]);
}

#[test]
fn refresh_replaces_model_with_current_database_state() {
    let mut user = User {
        id: 7,
        name: "Ada".into(),
    };
    let mut connection = FakeConnection::with_rows(vec![user_row(7, "Grace")]);

    let found = user.refresh(&mut connection).unwrap();

    assert!(found);
    assert_eq!(user.name, "Grace");
    assert_eq!(connection.statements[0].bindings(), &[Value::U64(7)]);
}

#[test]
fn refresh_returns_false_when_row_no_longer_exists() {
    let mut user = User {
        id: 7,
        name: "Ada".into(),
    };
    let mut connection = FakeConnection::default();

    let found = user.refresh(&mut connection).unwrap();

    assert!(!found);
    assert_eq!(user.name, "Ada");
    assert_eq!(connection.statements[0].bindings(), &[Value::U64(7)]);
}
