use claw_orm::{field, Model, Result, Row, Statement, Value};
use framework_database::{
    Capabilities, Column, Connection, DatabaseError, Driver, ErrorKind, Execution, Transaction,
    TransactionOptions,
};
use std::collections::VecDeque;

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
    query_results: VecDeque<Vec<Row>>,
    queried: Vec<Statement>,
}

impl FakeConnection {
    fn with_results(results: Vec<Vec<Row>>) -> Self {
        Self {
            query_results: results.into(),
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
        Ok(Execution::default())
    }

    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        self.queried.push(statement.clone());
        Ok(self.query_results.pop_front().unwrap_or_default())
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
fn pagination_executes_count_then_a_bounded_model_query() {
    let count = Row::new(vec![Column::new("aggregate", 3_u64)]).unwrap();
    let user = Row::new(vec![
        Column::new("id", 2_u64),
        Column::new("name", "B"),
    ])
    .unwrap();
    let mut connection = FakeConnection::with_results(vec![vec![count], vec![user]]);

    let page = User::query()
        .where_("active", "=", true)
        .paginate(&mut connection, 2, 1)
        .unwrap();

    assert_eq!(page.total(), 3);
    assert_eq!(page.last_page(), 3);
    assert!(page.has_previous() && page.has_next());
    assert_eq!(page.items()[0].name, "B");
    assert!(connection.queried[0]
        .sql()
        .starts_with("SELECT COUNT(*) AS \"aggregate\""));
    assert!(connection.queried[1].sql().ends_with("LIMIT 1 OFFSET 1"));
}
