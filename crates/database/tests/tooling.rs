use berserk_database::{
    run_seeders, Capabilities, Column, Connection, DatabaseError, Driver, ErrorKind, Execution,
    Factory, Migration, MigrationRunner, Result, Row, Seeder, Statement, Transaction,
    TransactionOptions,
};
use std::collections::VecDeque;

#[derive(Default)]
struct FakeConnection {
    query_results: VecDeque<Vec<Row>>,
    executed: Vec<Statement>,
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
    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        self.executed.push(statement.clone());
        Ok(Execution {
            affected_rows: 1,
            last_insert_id: None,
        })
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

struct CreateUsers;
impl Migration for CreateUsers {
    fn name(&self) -> &'static str {
        "202609120001_create_users"
    }
    fn up(&self, _driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![Statement::new(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        )])
    }
    fn down(&self, _driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![Statement::new("DROP TABLE users")])
    }
}

struct CreatePosts;
impl Migration for CreatePosts {
    fn name(&self) -> &'static str {
        "202609120002_create_posts"
    }
    fn up(&self, _driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![Statement::new(
            "CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL)",
        )])
    }
    fn down(&self, _driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![Statement::new("DROP TABLE posts")])
    }
}

fn applied_row(name: &str, batch: u64) -> Row {
    Row::new(vec![Column::new("name", name), Column::new("batch", batch)]).unwrap()
}

#[test]
fn migrations_are_tracked_only_after_their_up_steps() {
    let migration = CreateUsers;
    let runner = MigrationRunner::new([&migration as &dyn Migration]).unwrap();
    let mut connection = FakeConnection::with_results(vec![vec![]]);
    let report = runner.migrate(&mut connection).unwrap();

    assert_eq!(report.applied, vec!["202609120001_create_users"]);
    assert_eq!(connection.executed[0].sql(), "CREATE TABLE IF NOT EXISTS \"__framework_migrations\" (\"name\" TEXT PRIMARY KEY, \"batch\" INTEGER NOT NULL)");
    assert_eq!(
        connection.executed[1].sql(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)"
    );
    assert!(connection.executed[2]
        .sql()
        .starts_with("INSERT INTO \"__framework_migrations\""));
}

#[test]
fn rollback_uses_the_latest_batch_in_reverse_order() {
    let migration = CreateUsers;
    let runner = MigrationRunner::new([&migration as &dyn Migration]).unwrap();
    let mut connection = FakeConnection::with_results(vec![vec![applied_row(migration.name(), 3)]]);
    let report = runner.rollback_last(&mut connection).unwrap();
    assert_eq!(report.rolled_back, vec![migration.name()]);
    assert_eq!(connection.executed[1].sql(), "DROP TABLE users");
    assert!(connection.executed[2]
        .sql()
        .starts_with("DELETE FROM \"__framework_migrations\""));
}

#[test]
fn rollback_all_unwinds_batches_from_newest_to_oldest() {
    let users = CreateUsers;
    let posts = CreatePosts;
    let runner = MigrationRunner::new([
        &users as &dyn Migration,
        &posts as &dyn Migration,
    ])
    .unwrap();
    let mut connection = FakeConnection::with_results(vec![
        vec![applied_row(users.name(), 1), applied_row(posts.name(), 2)],
        vec![applied_row(users.name(), 1)],
        vec![],
    ]);

    let report = runner.rollback_all(&mut connection).unwrap();

    assert_eq!(report.rolled_back, vec![posts.name(), users.name()]);
    assert!(connection
        .executed
        .iter()
        .any(|statement| statement.sql() == "DROP TABLE posts"));
    assert!(connection
        .executed
        .iter()
        .any(|statement| statement.sql() == "DROP TABLE users"));
}

#[derive(Debug, PartialEq)]
struct User {
    id: u64,
    name: String,
}

struct BaseSeeder;
impl Seeder for BaseSeeder {
    fn name(&self) -> &'static str {
        "base"
    }
    fn run(&self, connection: &mut dyn Connection) -> Result<()> {
        connection
            .execute(&Statement::new("seed base data"))
            .map(|_| ())
    }
}

struct UserFactory;
impl Factory for UserFactory {
    type Output = User;
    fn make(&mut self, index: u64) -> User {
        User {
            id: index + 1,
            name: format!("User {}", index + 1),
        }
    }
    fn persist(&mut self, connection: &mut dyn Connection, user: &User) -> Result<()> {
        connection
            .execute(
                &Statement::new("insert factory user")
                    .bind(user.id)
                    .bind(user.name.clone()),
            )
            .map(|_| ())
    }
}

#[test]
fn seeders_are_ordered_and_factories_are_deterministic() {
    let mut connection = FakeConnection::default();
    assert_eq!(
        run_seeders(&mut connection, &[&BaseSeeder]).unwrap(),
        vec!["base"]
    );
    let users = UserFactory.create_many(&mut connection, 2).unwrap();
    assert_eq!(
        users[0],
        User {
            id: 1,
            name: "User 1".into()
        }
    );
    assert_eq!(
        users[1],
        User {
            id: 2,
            name: "User 2".into()
        }
    );
    assert_eq!(connection.executed.len(), 3);
}
