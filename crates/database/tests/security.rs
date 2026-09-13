use framework_database::{Statement, Value};

#[test]
fn database_debug_output_redacts_values_and_sql() {
    let value = Value::from("database-secret");
    assert!(!format!("{value:?}").contains("database-secret"));
    assert!(!format!("{:?}", value.as_ref()).contains("database-secret"));

    let statement =
        Statement::new("SELECT * FROM users WHERE token = 'sql-secret'").bind("binding-secret");
    let debug = format!("{statement:?}");
    assert!(!debug.contains("sql-secret"));
    assert!(!debug.contains("binding-secret"));
    assert!(debug.contains("binding_count"));
}
