#![cfg(feature = "mysql")]

mod common;

use berserk_database::{drivers::mysql::MySqlConnection, Connection, Statement, Value};

fn connection() -> Option<MySqlConnection> {
    let url = match std::env::var("FRAMEWORK_MYSQL_TEST_URL") {
        Ok(url) => url,
        Err(error) => {
            assert!(
                !matches!(
                    std::env::var("BERSERK_REQUIRE_LIVE_DATABASES").as_deref(),
                    Ok("1")
                ),
                "FRAMEWORK_MYSQL_TEST_URL is required for live database tests: {error}"
            );
            return None;
        }
    };
    Some(MySqlConnection::connect(&url).expect("test database must accept the connection"))
}

#[test]
fn mysql_executes_bound_queries_and_maps_rows() {
    let Some(mut connection) = connection() else {
        return;
    };
    connection.ping().unwrap();
    let rows = connection
        .query(
            &Statement::new(
                "select cast(? as signed) as id, cast(? as char) as name, null as optional",
            )
            .bind(42_i64)
            .bind("Ada"),
        )
        .unwrap();
    assert_eq!(rows[0].get("id"), Some(&Value::I64(42)));
    assert_eq!(rows[0].get("name"), Some(&Value::Text("Ada".into())));
    assert_eq!(rows[0].get("optional"), Some(&Value::Null));
}

#[test]
fn mysql_rolls_back_transactions() {
    let Some(mut connection) = connection() else {
        return;
    };
    let mut transaction = connection.begin(Default::default()).unwrap();
    let rows = transaction
        .query(&Statement::new("select 7 as value"))
        .unwrap();
    let value = rows[0].get("value");
    assert!(value == Some(&Value::I64(7)) || value == Some(&Value::U64(7)));
    transaction.rollback().unwrap();
}

#[test]
fn mysql_runs_the_live_migration_contract() {
    let Some(mut connection) = connection() else {
        return;
    };
    common::run_live_migration_contract(&mut connection);
}

#[test]
fn mysql_runs_the_live_database_contract() {
    let Some(mut connection) = connection() else {
        return;
    };
    common::run_live_database_contract(&mut connection);
}

#[test]
fn mysql_runs_the_live_transaction_contract() {
    let Some(mut connection) = connection() else {
        return;
    };
    common::run_live_transaction_contract(&mut connection);
}
