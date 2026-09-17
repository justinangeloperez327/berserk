use claw_orm::{field, Model, Result, Row, Statement, Value};
use berserk_database::{
    Capabilities, Column, Connection, DatabaseError, Driver, ErrorKind, Execution, Transaction,
    TransactionOptions,
};

struct User {
    id: u64,
}

impl Model for User {
    const TABLE: &'static str = "users";

    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
        })
    }

    fn key(&self) -> Value {
        self.id.into()
    }
}

struct FakeConnection {
    rows: Vec<Row>,
    statements: Vec<Statement>,
}

impl FakeConnection {
    fn with_count(total: u64) -> Self {
        Self {
            rows: vec![Row::new(vec![Column::new("aggregate", total)]).unwrap()],
            statements: Vec::new(),
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
        Ok(Execution::default())
    }

    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        self.statements.push(statement.clone());
        Ok(std::mem::take(&mut self.rows))
    }

    fn begin(&mut self, _options: TransactionOptions) -> Result<Box<dyn Transaction + '_>> {
        Err(DatabaseError::new(
            ErrorKind::Transaction,
            "not supported by fake",
        ))
    }

    fn ping(&mut self) -> Result<()> {
        Ok(())
    }
}

#[test]
fn model_and_filtered_query_count_use_the_database_count_path() {
    let mut all = FakeConnection::with_count(3);
    assert_eq!(User::count(&mut all).unwrap(), 3);
    assert_eq!(
        all.statements[0].sql(),
        "SELECT COUNT(*) AS \"aggregate\" FROM \"users\""
    );
    assert!(all.statements[0].bindings().is_empty());

    let mut active = FakeConnection::with_count(2);
    assert_eq!(
        User::where_("active", "=", true)
            .count(&mut active)
            .unwrap(),
        2
    );
    assert_eq!(
        active.statements[0].sql(),
        "SELECT COUNT(*) AS \"aggregate\" FROM \"users\" WHERE \"active\" = ?"
    );
    assert_eq!(active.statements[0].bindings(), &[Value::Bool(true)]);
}
