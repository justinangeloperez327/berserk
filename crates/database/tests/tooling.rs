use framework_database::{
    field, run_seeders, Capabilities, Column, Connection, DatabaseError, Driver, ErrorKind,
    Execution, Factory, Migration, MigrationRunner, Model, Result, Row, Seeder, Statement,
    Transaction, TransactionOptions, Value,
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

#[test]
fn pagination_executes_count_then_a_bounded_query() {
    let count = Row::new(vec![Column::new("aggregate", 3_u64)]).unwrap();
    let user = Row::new(vec![Column::new("id", 2_u64), Column::new("name", "B")]).unwrap();
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
