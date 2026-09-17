#![cfg(feature = "postgres")]

mod common;

use berserk_database::{drivers::postgres::PostgresConnection, Connection, Statement, Value};

fn connection() -> Option<PostgresConnection> {
    let url = match std::env::var("FRAMEWORK_POSTGRES_TEST_URL") {
        Ok(url) => url,
        Err(error) => {
            assert!(
                !matches!(
                    std::env::var("BERSERK_REQUIRE_LIVE_DATABASES").as_deref(),
                    Ok("1")
                ),
                "FRAMEWORK_POSTGRES_TEST_URL is required for live database tests: {error}"
            );
            return None;
        }
    };
    Some(
        PostgresConnection::connect_no_tls(&url).expect("test database must accept the connection"),
    )
}

#[test]
fn postgres_executes_bound_queries_and_maps_rows() {
    let Some(mut connection) = connection() else {
        return;
    };
    connection.ping().unwrap();

    let rows = connection
        .query(
            &Statement::new("select $1::bigint as id, $2::text as name, null::text as optional")
                .bind(42_i64)
                .bind("Ada"),
        )
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("id"), Some(&Value::I64(42)));
    assert_eq!(rows[0].get("name"), Some(&Value::Text("Ada".into())));
    assert_eq!(rows[0].get("optional"), Some(&Value::Null));
}

#[test]
fn postgres_executes_and_rolls_back_transactions() {
    let Some(mut connection) = connection() else {
        return;
    };
    let mut transaction = connection.begin(Default::default()).unwrap();
    let rows = transaction
        .query(&Statement::new("select 7::integer as value"))
        .unwrap();
    assert_eq!(rows[0].get("value"), Some(&Value::I64(7)));
    transaction.rollback().unwrap();
}

#[test]
fn postgres_runs_the_live_migration_contract() {
    let Some(mut connection) = connection() else {
        return;
    };
    common::run_live_migration_contract(&mut connection);
}
