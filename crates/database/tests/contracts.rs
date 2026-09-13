use framework_database::{
    Capabilities, Capability, Column, Driver, ErrorKind, Row, Statement, Value,
};

#[test]
fn statements_keep_sql_separate_from_bindings() {
    let statement = Statement::new("select * from users where id = ?").bind(42_i64);
    assert_eq!(statement.sql(), "select * from users where id = ?");
    assert_eq!(statement.bindings(), &[Value::I64(42)]);
}

#[test]
fn rows_reject_duplicate_names() {
    let result = Row::new(vec![Column::new("id", 1_i64), Column::new("id", 2_i64)]);
    assert!(matches!(result.unwrap_err().kind(), ErrorKind::Decode));
}

#[test]
fn capabilities_are_explicit() {
    let capabilities = Capabilities::new().with(Capability::Returning);
    assert!(capabilities.supports(Capability::Returning));
    assert!(!capabilities.supports(Capability::AdvisoryLocks));
    assert_eq!(
        framework_database::driver_enabled(Driver::Postgres),
        cfg!(feature = "postgres")
    );
}
