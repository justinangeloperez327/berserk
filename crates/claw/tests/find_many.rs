use claw_orm::{field, Model, Result, Row, Statement, Value};
use berserk_database::{
    Capabilities, Column, Connection, DatabaseError, Driver, ErrorKind, Execution, Transaction,
    TransactionOptions,
};

#[derive(Debug, PartialEq)]
struct User {
    user_id: u64,
    name: String,
}

impl Model for User {
    const TABLE: &'static str = "users";
    const PRIMARY_KEY: &'static str = "user_id";

    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            user_id: field(row, "user_id")?,
            name: field(row, "name")?,
        })
    }

    fn key(&self) -> Value {
        self.user_id.into()
    }
}

struct FakeConnection {
    rows: Vec<Row>,
    statements: Vec<Statement>,
}

impl FakeConnection {
    fn with_users() -> Self {
        Self {
            rows: vec![
                Row::new(vec![
                    Column::new("user_id", 3_u64),
                    Column::new("name", "Ada"),
                ])
                .unwrap(),
                Row::new(vec![
                    Column::new("user_id", 8_u64),
                    Column::new("name", "Grace"),
                ])
                .unwrap(),
            ],
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
fn find_many_uses_the_declared_primary_key_and_bound_values() {
    let mut connection = FakeConnection::with_users();
    let users = User::find_many(&mut connection, [3_u64, 8_u64]).unwrap();

    assert_eq!(
        users,
        vec![
            User {
                user_id: 3,
                name: "Ada".into(),
            },
            User {
                user_id: 8,
                name: "Grace".into(),
            },
        ]
    );
    assert_eq!(connection.statements.len(), 1);
    assert_eq!(
        connection.statements[0].sql(),
        "SELECT * FROM \"users\" WHERE \"user_id\" IN (?, ?)"
    );
    assert_eq!(
        connection.statements[0].bindings(),
        &[Value::U64(3), Value::U64(8)]
    );
}
