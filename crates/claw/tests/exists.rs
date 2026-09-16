use claw_orm::{field, Model, Result, Row, Statement, Value};
use framework_database::{
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
    fn with_rows(rows: Vec<Row>) -> Self {
        Self {
            rows,
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
fn exists_uses_a_bounded_query_without_decoding_models() {
    let sentinel = Row::new(vec![Column::new("sentinel", true)]).unwrap();
    let mut any_user = FakeConnection::with_rows(vec![sentinel]);

    assert!(User::exists(&mut any_user).unwrap());
    assert_eq!(
        any_user.statements[0].sql(),
        "SELECT * FROM \"users\" LIMIT 1"
    );
    assert!(any_user.statements[0].bindings().is_empty());

    let mut active_user = FakeConnection::with_rows(Vec::new());
    assert!(!User::where_("active", "=", true)
        .exists(&mut active_user)
        .unwrap());
    assert_eq!(
        active_user.statements[0].sql(),
        "SELECT * FROM \"users\" WHERE \"active\" = ? LIMIT 1"
    );
    assert_eq!(
        active_user.statements[0].bindings(),
        &[Value::Bool(true)]
    );
}
