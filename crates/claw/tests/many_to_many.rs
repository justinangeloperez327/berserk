use berserk_database::{
    Capabilities, Column, Connection, DatabaseError, Driver, Execution, Transaction,
    TransactionOptions,
};
use claw_orm::{field, BelongsToMany, Model, Result, Row, Statement, Value};
use std::collections::VecDeque;

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
struct Role {
    id: u64,
    name: String,
}

impl Model for Role {
    const TABLE: &'static str = "roles";

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
    results: VecDeque<Vec<Row>>,
    statements: Vec<Statement>,
}

impl FakeConnection {
    fn with_results(results: impl IntoIterator<Item = Vec<Row>>) -> Self {
        Self {
            results: results.into_iter().collect(),
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
        Ok(Execution {
            affected_rows: 0,
            last_insert_id: None,
        })
    }

    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        self.statements.push(statement.clone());
        Ok(self.results.pop_front().unwrap_or_default())
    }

    fn begin(&mut self, _options: TransactionOptions) -> Result<Box<dyn Transaction + '_>> {
        Err(DatabaseError::new(
            claw_orm::ErrorKind::Transaction,
            "not supported by fake",
        ))
    }

    fn ping(&mut self) -> Result<()> {
        Ok(())
    }
}

fn pivot(user_id: u64, role_id: u64) -> Row {
    Row::new(vec![
        Column::new("user_id", user_id),
        Column::new("role_id", role_id),
    ])
    .unwrap()
}

fn role(id: u64, name: &str) -> Row {
    Row::new(vec![Column::new("id", id), Column::new("name", name)]).unwrap()
}

fn relation() -> BelongsToMany<User, Role> {
    BelongsToMany::new("role_user", "user_id", "role_id", User::key, Role::key)
}

#[test]
fn many_to_many_batches_pivot_and_related_queries() {
    let users = [User { id: 1 }, User { id: 2 }];
    let mut connection = FakeConnection::with_results([
        vec![pivot(1, 10), pivot(1, 11), pivot(2, 11)],
        vec![role(10, "Editor"), role(11, "Admin")],
    ]);

    let loaded = relation().load(&mut connection, &users).unwrap();

    assert_eq!(connection.statements.len(), 2);
    assert_eq!(loaded.get(&Value::U64(1)).unwrap().len(), 2);
    assert_eq!(loaded.get(&Value::U64(2)).unwrap().len(), 1);
    assert_eq!(loaded.get(&Value::U64(2)).unwrap()[0].name, "Admin");
}

#[test]
fn many_to_many_preserves_a_related_model_shared_by_multiple_parents() {
    let users = [User { id: 1 }, User { id: 2 }];
    let mut connection =
        FakeConnection::with_results([vec![pivot(1, 10), pivot(2, 10)], vec![role(10, "Admin")]]);

    let loaded = relation().load(&mut connection, &users).unwrap();

    assert_eq!(loaded.get(&Value::U64(1)).unwrap()[0].name, "Admin");
    assert_eq!(loaded.get(&Value::U64(2)).unwrap()[0].name, "Admin");
    assert_eq!(connection.statements.len(), 2);
}

#[test]
fn many_to_many_empty_parents_do_not_query() {
    let mut connection = FakeConnection::default();
    let loaded = relation().load(&mut connection, &[]).unwrap();

    assert!(loaded.is_empty());
    assert!(connection.statements.is_empty());
}
