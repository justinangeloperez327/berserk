#![cfg(feature = "sqlite")]

mod common;

use berserk_database::{
    drivers::sqlite::SqliteConnection, Connection, Direction, ErrorKind, Query, Statement,
    TransactionOptions, Value,
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
