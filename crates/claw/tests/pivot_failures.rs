use berserk_database::{Capabilities, Column, Execution, Transaction, TransactionOptions};
use claw_orm::{
    BelongsToMany, Connection, DatabaseError, Driver, ErrorKind, Model, Result, Row, Statement,
    Value,
};

struct User;
struct Role;
impl Model for Role {
    const TABLE: &'static str = "roles";
    fn from_row(_: &Row) -> Result<Self> {
        Ok(Self)
    }
    fn key(&self) -> Value {
        10.into()
    }
}
fn relation() -> BelongsToMany<User, Role> {
    BelongsToMany::new(
        "memberships",
        "account_id",
        "permission_id",
        |_| 7.into(),
        Role::key,
    )
}
struct Recording {
    driver: Driver,
    fail: &'static str,
    events: Vec<&'static str>,
    statements: Vec<Statement>,
    malformed: Option<Row>,
}
impl Recording {
    fn new(driver: Driver, fail: &'static str) -> Self {
        Self {
            driver,
            fail,
            events: vec![],
            statements: vec![],
            malformed: None,
        }
    }
    fn event(&mut self, event: &'static str) -> Result<()> {
        self.events.push(event);
        if self.fail == event || (self.fail == "rollback" && event == "insert") {
            Err(DatabaseError::new(ErrorKind::Transaction, event).with_code(event))
        } else {
            Ok(())
        }
    }
}
impl Connection for Recording {
    fn driver(&self) -> Driver {
        self.driver
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
    }
    fn execute(&mut self, s: &Statement) -> Result<Execution> {
        self.statements.push(s.clone());
        self.event(if s.sql().starts_with("INSERT") {
            "insert"
        } else {
            "delete"
        })?;
        Ok(Execution {
            affected_rows: 1,
            last_insert_id: None,
        })
    }
    fn query(&mut self, s: &Statement) -> Result<Vec<Row>> {
        self.statements.push(s.clone());
        self.event("query")?;
        Ok(vec![self.malformed.take().unwrap_or_else(|| {
            Row::new(vec![Column::new("permission_id", 10)]).unwrap()
        })])
    }
    fn begin(&mut self, _: TransactionOptions) -> Result<Box<dyn Transaction + '_>> {
        self.event("begin")?;
        Ok(Box::new(Tx(self)))
    }
    fn ping(&mut self) -> Result<()> {
        Ok(())
    }
}
struct Tx<'a>(&'a mut Recording);
impl Transaction for Tx<'_> {
    fn execute(&mut self, s: &Statement) -> Result<Execution> {
        self.0.execute(s)
    }
    fn query(&mut self, s: &Statement) -> Result<Vec<Row>> {
        self.0.query(s)
    }
    fn commit(self: Box<Self>) -> Result<()> {
        self.0.event("commit")
    }
    fn rollback(self: Box<Self>) -> Result<()> {
        self.0.event("rollback")
    }
}

#[test]
fn sync_begins_before_read_and_commits_after_all_writes() {
    let mut c = Recording::new(Driver::Postgres, "");
    relation().sync_on(&mut c, &User, [11]).unwrap();
    assert_eq!(c.events, ["begin", "query", "delete", "insert", "commit"]);
    assert_eq!(
        c.statements[0].sql(),
        "SELECT \"permission_id\" FROM \"memberships\" WHERE \"account_id\" = $1"
    );
    assert_eq!(c.statements[1].bindings(), [Value::I64(7), Value::U64(10)]);
    assert_eq!(c.statements[2].bindings(), [Value::I64(7), Value::U64(11)]);
}

#[test]
fn begin_query_mutation_commit_and_rollback_failures_propagate() {
    for (failure, events) in [
        ("begin", vec!["begin"]),
        ("query", vec!["begin", "query", "rollback"]),
        ("delete", vec!["begin", "query", "delete", "rollback"]),
        (
            "insert",
            vec!["begin", "query", "delete", "insert", "rollback"],
        ),
        (
            "commit",
            vec!["begin", "query", "delete", "insert", "commit"],
        ),
        (
            "rollback",
            vec!["begin", "query", "delete", "insert", "rollback"],
        ),
    ] {
        let mut c = Recording::new(Driver::Postgres, failure);
        let error = relation().sync_on(&mut c, &User, [11]).unwrap_err();
        assert_eq!(error.code(), Some(failure));
        assert_eq!(c.events, events);
    }
}

#[test]
fn malformed_current_state_rolls_back_without_mutations() {
    for row in [
        Row::new(vec![]).unwrap(),
        Row::new(vec![Column::new("permission_id", Value::Null)]).unwrap(),
    ] {
        let mut c = Recording::new(Driver::Sqlite, "");
        c.malformed = Some(row);
        assert_eq!(
            relation().sync_on(&mut c, &User, [11]).unwrap_err().kind(),
            &ErrorKind::Decode
        );
        assert_eq!(c.events, ["begin", "query", "rollback"]);
    }
}

#[test]
fn mutations_quote_configured_identifiers_and_bind_values_for_every_driver() {
    for driver in [Driver::Sqlite, Driver::Postgres, Driver::MySql] {
        let mut c = Recording::new(driver, "");
        let hostile = "x'); DROP TABLE memberships; --";
        relation().attach_on(&mut c, &User, hostile).unwrap();
        relation().detach_on(&mut c, &User, hostile).unwrap();
        relation().detach_all_on(&mut c, &User).unwrap();
        let q = if driver == Driver::MySql { '`' } else { '"' };
        for statement in &c.statements {
            assert!(statement.sql().contains(&format!("{q}memberships{q}")));
            assert!(statement.sql().contains(&format!("{q}account_id{q}")));
            assert!(!statement.sql().contains(hostile));
            assert_eq!(statement.bindings()[0], Value::I64(7));
        }
        for statement in &c.statements[..2] {
            assert!(statement.sql().contains(&format!("{q}permission_id{q}")));
            assert_eq!(statement.bindings()[1], Value::Text(hostile.into()));
            assert!(statement.sql().contains(if driver == Driver::Postgres {
                "$2"
            } else {
                "?"
            }));
        }
    }
}

#[test]
fn invalid_identifiers_never_reach_the_database() {
    let mut c = Recording::new(Driver::Sqlite, "");
    let relation = BelongsToMany::<User, Role>::new(
        "memberships; DROP TABLE users",
        "account_id",
        "permission_id",
        |_| 7.into(),
        Role::key,
    );
    assert_eq!(
        relation.attach_on(&mut c, &User, 10).unwrap_err().kind(),
        &ErrorKind::Query
    );
    assert!(c.statements.is_empty());
}
