#![cfg(feature = "sqlite")]

use std::cell::Cell;
use berserk_database::{drivers::sqlite::SqliteConnection, Capabilities, Connection, Execution, Statement, Transaction, TransactionOptions};
use claw_orm::{field, HasMany, Model, Result, Row, Value};

#[derive(Debug)] struct Parent(i64);
#[derive(Debug)] struct Child { id: i64, parent_id: i64 }
impl Model for Parent {
    const TABLE: &'static str = "parents";
    fn from_row(row: &Row) -> Result<Self> { Ok(Self(field(row, "id")?)) }
    fn key(&self) -> Value { self.0.into() }
}
impl Model for Child {
    const TABLE: &'static str = "children";
    fn from_row(row: &Row) -> Result<Self> { Ok(Self { id: field(row, "id")?, parent_id: field(row, "parent_id")? }) }
    fn key(&self) -> Value { self.id.into() }
}
fn children() -> HasMany<Parent, Child> { HasMany::new("parent_id", Parent::key, |c| c.parent_id.into()) }

struct CountingConnection { inner: SqliteConnection, queries: Cell<usize> }
impl Connection for CountingConnection {
    fn driver(&self) -> berserk_database::Driver { self.inner.driver() }
    fn capabilities(&self) -> Capabilities { self.inner.capabilities() }
    fn execute(&mut self, statement: &Statement) -> Result<Execution> { self.inner.execute(statement) }
    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> { self.queries.set(self.queries.get()+1); self.inner.query(statement) }
    fn begin(&mut self, options: TransactionOptions) -> Result<Box<dyn Transaction + '_>> { self.inner.begin(options) }
    fn ping(&mut self) -> Result<()> { self.inner.ping() }
}

#[test]
fn typed_eager_loading_is_constant_query_count_for_parent_batch() {
    let mut inner = SqliteConnection::in_memory().unwrap();
    inner.execute(&Statement::new("CREATE TABLE parents (id INTEGER PRIMARY KEY)")).unwrap();
    inner.execute(&Statement::new("CREATE TABLE children (id INTEGER PRIMARY KEY, parent_id INTEGER NOT NULL)")).unwrap();
    inner.execute(&Statement::new("INSERT INTO parents VALUES (1), (2), (3), (4)")).unwrap();
    inner.execute(&Statement::new("INSERT INTO children VALUES (10,1),(11,1),(12,2),(13,4)")).unwrap();
    let mut c = CountingConnection { inner, queries: Cell::new(0) };

    let loaded = Parent::query().with(children()).get_on(&mut c).unwrap();
    assert_eq!(loaded.models.len(), 4);
    assert_eq!(loaded.relations.get(&Value::I64(1)).unwrap().len(), 2);
    assert_eq!(c.queries.get(), 2, "one parent query plus one batched relationship query");
}
