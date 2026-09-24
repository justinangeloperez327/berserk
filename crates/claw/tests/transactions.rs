#![cfg(feature = "sqlite")]

use berserk_database::{
    drivers::sqlite::SqliteConnection, scope::with_scoped_connection, Connection, ErrorKind,
    Statement, Value,
};
use claw_orm::{DatabaseError, Transaction};

#[test]
fn scoped_transaction_commits_and_rolls_back_application_errors() {
    let mut connection = SqliteConnection::in_memory().unwrap();
    connection
        .execute(&Statement::new(
            "CREATE TABLE tx_items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        ))
        .unwrap();

    with_scoped_connection(&mut connection, || {
        Transaction::run(|| -> claw_orm::Result<()> {
            berserk_database::scope::with_connection(|connection| {
                connection.execute(
                    &Statement::new("INSERT INTO tx_items VALUES (?1, ?2)")
                        .bind(1_i64)
                        .bind("commit"),
                )?;
                Ok(())
            })
        })
        .unwrap();

        let result: claw_orm::Result<()> = Transaction::run(|| {
            berserk_database::scope::with_connection(|connection| {
                connection.execute(
                    &Statement::new("INSERT INTO tx_items VALUES (?1, ?2)")
                        .bind(2_i64)
                        .bind("rollback"),
                )?;
                Ok(())
            })?;
            Err(DatabaseError::new(
                ErrorKind::InvalidInput,
                "application failure",
            ))
        });
        assert!(result.is_err());
    });

    let rows = connection
        .query(&Statement::new("SELECT id FROM tx_items ORDER BY id"))
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("id"), Some(&Value::I64(1)));
}

#[test]
fn nested_scoped_transaction_fails_before_inner_work() {
    let mut connection = SqliteConnection::in_memory().unwrap();
    connection
        .execute(&Statement::new(
            "CREATE TABLE tx_nested (id INTEGER PRIMARY KEY)",
        ))
        .unwrap();

    with_scoped_connection(&mut connection, || {
        let outer: claw_orm::Result<()> = Transaction::run(|| {
            let nested: claw_orm::Result<()> = Transaction::run(|| -> claw_orm::Result<()> {
                panic!("nested operation must not execute");
            });
            let error = nested.unwrap_err();
            assert_eq!(error.kind(), &ErrorKind::Transaction);
            Err(DatabaseError::new(
                ErrorKind::InvalidInput,
                "rollback outer",
            ))
        });
        assert!(outer.is_err());
    });

    assert!(connection
        .query(&Statement::new("SELECT id FROM tx_nested"))
        .unwrap()
        .is_empty());
}
