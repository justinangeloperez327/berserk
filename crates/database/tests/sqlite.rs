#![cfg(feature = "sqlite")]

mod common;

use berserk_database::{
    drivers::sqlite::SqliteConnection, Connection, Direction, ErrorKind, Query, Statement,
    TransactionOptions, Value,
};
use berserk_database::{
    migrations::{compile_rebuild, Column, Table},
    Migration, MigrationRunner,
};

#[test]
fn sqlite_executes_bound_crud_and_maps_rows() {
    let mut connection = SqliteConnection::in_memory().unwrap();
    connection.ping().unwrap();
    connection
        .execute(&Statement::new(
            "create table users (id integer primary key, name text, data blob)",
        ))
        .unwrap();
    let result = connection
        .execute(
            &Statement::new("insert into users (id, name, data) values (?1, ?2, ?3)")
                .bind(1_i64)
                .bind("Ada")
                .bind(vec![0_u8, 255]),
        )
        .unwrap();
    assert_eq!(result.affected_rows, 1);

    let rows = connection
        .query(&Statement::new("select id, name, data from users where id = ?1").bind(1_i64))
        .unwrap();
    assert_eq!(rows[0].get("id"), Some(&Value::I64(1)));
    assert_eq!(rows[0].get("name"), Some(&Value::Text("Ada".into())));
    assert_eq!(rows[0].get("data"), Some(&Value::Bytes(vec![0, 255])));
}

#[test]
fn sqlite_rolls_back_and_rejects_read_only_emulation() {
    let mut connection = SqliteConnection::in_memory().unwrap();
    connection
        .execute(&Statement::new("create table changes (value integer)"))
        .unwrap();
    let mut transaction = connection.begin(Default::default()).unwrap();
    transaction
        .execute(&Statement::new("insert into changes values (?1)").bind(7_i64))
        .unwrap();
    transaction.rollback().unwrap();
    assert!(connection
        .query(&Statement::new("select value from changes"))
        .unwrap()
        .is_empty());

    let error = match connection.begin(TransactionOptions { read_only: true }) {
        Ok(_) => panic!("read-only SQLite transaction unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(matches!(error.kind(), ErrorKind::Unsupported { .. }));
}

#[test]
fn fluent_queries_execute_against_sqlite() {
    let mut connection = SqliteConnection::in_memory().unwrap();
    connection
        .execute(&Statement::new(
            "create table users (id integer, name text)",
        ))
        .unwrap();
    Query::table("users")
        .insert(vec![("id", Value::I64(2)), ("name", Value::from("B"))])
        .execute(&mut connection)
        .unwrap();
    Query::table("users")
        .insert(vec![("id", Value::I64(1)), ("name", Value::from("A"))])
        .execute(&mut connection)
        .unwrap();

    let rows = Query::table("users")
        .select(["id", "name"])
        .where_("id", ">=", 1_i64)
        .order_by("id", Direction::Asc)
        .limit(1)
        .get(&mut connection)
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("id"), Some(&Value::I64(1)));

    Query::table("users")
        .where_("id", "=", 1_i64)
        .update([("name", "Ada")])
        .execute(&mut connection)
        .unwrap();
    Query::table("users")
        .where_("id", "=", 2_i64)
        .delete()
        .execute(&mut connection)
        .unwrap();
    assert_eq!(Query::table("users").get(&mut connection).unwrap().len(), 1);
}

#[test]
fn sqlite_runs_the_live_migration_contract() {
    let mut connection = SqliteConnection::in_memory().unwrap();
    common::run_live_migration_contract(&mut connection);
}

#[test]
fn sqlite_runs_the_live_database_contract() {
    let mut connection = SqliteConnection::in_memory().unwrap();
    connection
        .execute(&Statement::new("PRAGMA foreign_keys = ON"))
        .unwrap();
    common::run_live_database_contract(&mut connection);
}

#[test]
fn sqlite_runs_the_live_transaction_contract() {
    let mut connection = SqliteConnection::in_memory().unwrap();
    common::run_live_transaction_contract(&mut connection);
}

struct RenameUser;
impl Migration for RenameUser {
    fn name(&self) -> &'static str {
        "rename_user_name"
    }
    fn up(&self, driver: berserk_database::Driver) -> berserk_database::Result<Vec<Statement>> {
        compile_rebuild(
            &Table::rebuild(
                "users",
                Table::create("users").columns([Column::id(), Column::string("full_name")]),
            )
            .copy([("id", "id"), ("name", "full_name")]),
            driver,
        )
    }
    fn down(&self, _driver: berserk_database::Driver) -> berserk_database::Result<Vec<Statement>> {
        Ok(vec![Statement::new("DROP TABLE users")])
    }
}

#[test]
fn sqlite_rebuild_rejects_referenced_parents_before_changing_data() {
    for action in ["CASCADE", "SET NULL", "NO ACTION"] {
        let mut connection = SqliteConnection::in_memory().unwrap();
        for sql in [
            "PRAGMA foreign_keys = ON".to_owned(),
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)".to_owned(),
            format!("CREATE TABLE tasks (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id) ON DELETE {action})"),
            "INSERT INTO users VALUES (1, 'Ada')".to_owned(),
            "INSERT INTO tasks VALUES (10, 1)".to_owned(),
        ] {
            connection.execute(&Statement::new(sql)).unwrap();
        }
        let migration = RenameUser;
        let runner = MigrationRunner::new([&migration as &dyn Migration]).unwrap();
        assert_eq!(
            runner.migrate(&mut connection).unwrap_err().kind(),
            &ErrorKind::InvalidInput
        );
        assert!(runner.applied(&mut connection).unwrap().is_empty());
        let children = connection
            .query(&Statement::new("SELECT user_id FROM tasks"))
            .unwrap();
        assert_eq!(children[0].get("user_id"), Some(&Value::I64(1)));
        assert_eq!(
            connection
                .query(&Statement::new("SELECT name FROM users"))
                .unwrap()[0]
                .get("name"),
            Some(&Value::from("Ada"))
        );
        assert!(connection
            .query(&Statement::new("SELECT * FROM __br_users"))
            .is_err());
    }
}

#[test]
fn sqlite_rebuild_preserves_unreferenced_table_rows() {
    let mut connection = SqliteConnection::in_memory().unwrap();
    connection
        .execute(&Statement::new("PRAGMA foreign_keys = ON"))
        .unwrap();
    connection
        .execute(&Statement::new(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        ))
        .unwrap();
    connection
        .execute(&Statement::new("INSERT INTO users VALUES (1, 'Ada')"))
        .unwrap();
    let migration = RenameUser;
    MigrationRunner::new([&migration as &dyn Migration])
        .unwrap()
        .migrate(&mut connection)
        .unwrap();
    let rows = connection
        .query(&Statement::new("SELECT id, full_name FROM users"))
        .unwrap();
    assert_eq!(rows[0].get("id"), Some(&Value::I64(1)));
    assert_eq!(rows[0].get("full_name"), Some(&Value::from("Ada")));
}
