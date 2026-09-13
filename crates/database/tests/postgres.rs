#![cfg(feature = "postgres")]

use framework_database::{drivers::postgres::PostgresConnection, Connection, Statement, Value};

fn connection() -> Option<PostgresConnection> {
    let url = std::env::var("FRAMEWORK_POSTGRES_TEST_URL").ok()?;
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
