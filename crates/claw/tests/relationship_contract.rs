#![cfg(feature = "sqlite")]

use berserk_database::{drivers::sqlite::SqliteConnection, Connection, Statement, Value};
use claw_orm::{field, BelongsTo, HasMany, HasOne, Model, Result, Row};

#[derive(Debug)] struct Parent(i64);
#[derive(Debug)] struct Child { id: i64, parent_id: Option<i64> }
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
fn children() -> HasMany<Parent, Child> { HasMany::new("parent_id", Parent::key, |c| c.parent_id.map(Value::from).unwrap_or(Value::Null)) }
fn one_child() -> HasOne<Parent, Child> { HasOne::new("parent_id", Parent::key, |c| c.parent_id.map(Value::from).unwrap_or(Value::Null)) }
fn parent() -> BelongsTo<Child, Parent> { BelongsTo::new("id", |c| c.parent_id.map(Value::from), Parent::key) }

#[test]
fn relationships_batch_parents_and_preserve_cardinality() {
    let mut c = SqliteConnection::in_memory().unwrap();
    c.execute(&Statement::new("CREATE TABLE parents (id INTEGER PRIMARY KEY)")).unwrap();
    c.execute(&Statement::new("CREATE TABLE children (id INTEGER PRIMARY KEY, parent_id INTEGER NULL)")).unwrap();
    c.execute(&Statement::new("INSERT INTO parents VALUES (1), (2), (3)")).unwrap();
    c.execute(&Statement::new("INSERT INTO children VALUES (10, 1), (11, 1), (12, 2), (13, NULL)")).unwrap();

    let parents = [Parent(1), Parent(2), Parent(3), Parent(1)];
    let loaded = children().load_on(&mut c, &parents).unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded.get(&Value::I64(1)).unwrap().len(), 2);
    assert_eq!(loaded.get(&Value::I64(2)).unwrap().len(), 1);
    assert!(loaded.get(&Value::I64(3)).is_none());

    let kids = [
        Child { id: 10, parent_id: Some(1) },
        Child { id: 12, parent_id: Some(2) },
        Child { id: 13, parent_id: None },
    ];
    let owners = parent().load_on(&mut c, &kids).unwrap();
    assert_eq!(owners.len(), 2);
    assert_eq!(owners.get(&Value::I64(1)).unwrap().len(), 1);
    assert!(owners.get(&Value::Null).is_none());

    let error = one_child().load_on(&mut c, &parents).unwrap_err();
    assert_eq!(error.kind(), &berserk_database::ErrorKind::Decode);
}

#[test]
fn signed_and_unsigned_relationship_keys_match() {
    let mut c = SqliteConnection::in_memory().unwrap();
    c.execute(&Statement::new("CREATE TABLE parents (id INTEGER PRIMARY KEY)")).unwrap();
    c.execute(&Statement::new("CREATE TABLE children (id INTEGER PRIMARY KEY, parent_id INTEGER NULL)")).unwrap();
    c.execute(&Statement::new("INSERT INTO children VALUES (10, 7)")).unwrap();
    let loaded = children().load_on(&mut c, &[Parent(7)]).unwrap();
    assert_eq!(loaded.get(&Value::U64(7)).unwrap().len(), 1);
}
