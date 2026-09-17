use claw_orm::{field, Model, PersistableModel, Result, Row, Statement, Value};
use berserk_database::{
    Capabilities, Column, Connection, DatabaseError, Driver, ErrorKind, Execution, Transaction,
    TransactionOptions,
};

#[derive(Debug)]
struct User {
    id: u64,
    name: String,
    active: bool,
}

impl Model for User {
    const TABLE: &'static str = "users";

    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            name: field(row, "name")?,
            active: field(row, "active")?,
        })
    }

    fn key(&self) -> Value {
        self.id.into()
    }
}

impl PersistableModel for User {
    fn values_for_save(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("name", self.name.clone().into()),
            ("active", self.active.into()),
        ]
    }
}

struct BrokenUser {
    id: u64,
}

impl Model for BrokenUser {
    const TABLE: &'static str = "users";

    fn from_row(_row: &Row) -> Result<Self> {
        unreachable!("row decoding is not used by this test")
    }

    fn key(&self) -> Value {
        self.id.into()
    }
}

impl PersistableModel for BrokenUser {
    fn values_for_save(&self) -> Vec<(&'static str, Value)> {
        vec![("id", self.id.into())]
    }
}

struct EmptyUser {
    id: u64,
}

impl Model for EmptyUser {
    const TABLE: &'static str = "users";

    fn from_row(_row: &Row) -> Result<Self> {
        unreachable!("row decoding is not used by this test")
    }

    fn key(&self) -> Value {
        self.id.into()
    }
}

impl PersistableModel for EmptyUser {
    fn values_for_save(&self) -> Vec<(&'static str, Value)> {
        Vec::new()
    }
}

#[derive(Default)]
struct FakeConnection {
    executed: Vec<Statement>,
}

impl Connection for FakeConnection {
    fn driver(&self) -> Driver {
        Driver::Sqlite
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
    }

    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        self.executed.push(statement.clone());
        Ok(Execution {
            affected_rows: 1,
            last_insert_id: None,
        })
    }

    fn query(&mut self, _statement: &Statement) -> Result<Vec<Row>> {
        Ok(vec![Row::new(vec![Column::new("id", 1_u64)])?])
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

#[test]
fn save_persists_declared_values_and_filters_by_primary_key() {
    let user = User {
        id: 7,
        name: "Grace".into(),
        active: false,
    };
    let mut connection = FakeConnection::default();

    let execution = user.save(&mut connection).unwrap();

    assert_eq!(execution.affected_rows, 1);
    assert_eq!(connection.executed.len(), 1);
    assert_eq!(
        connection.executed[0].sql(),
        "UPDATE \"users\" SET \"name\" = ?, \"active\" = ? WHERE \"id\" = ?"
    );
    assert_eq!(
        connection.executed[0].bindings(),
        &[
            Value::Text("Grace".into()),
            Value::Bool(false),
            Value::U64(7),
        ]
    );
}

#[test]
fn save_rejects_primary_key_updates_before_execution() {
    let user = BrokenUser { id: 7 };
    let mut connection = FakeConnection::default();

    let error = user.save(&mut connection).unwrap_err();

    assert!(matches!(error.kind(), ErrorKind::Query));
    assert!(connection.executed.is_empty());
}

#[test]
fn save_rejects_models_without_persisted_values() {
    let user = EmptyUser { id: 7 };
    let mut connection = FakeConnection::default();

    let error = user.save(&mut connection).unwrap_err();

    assert!(matches!(error.kind(), ErrorKind::Query));
    assert!(connection.executed.is_empty());
}
