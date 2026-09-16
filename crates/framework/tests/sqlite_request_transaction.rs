#![cfg(feature = "sqlite")]

use framework::{
    database::{
        drivers::sqlite::SqliteConnection, Database, DatabaseError, ErrorKind, Query, Row,
        TransactionOptions, Value,
    },
    App, ConfigError, Error, Headers, Method, Request, Response, Result,
};

fn database() -> Database {
    Database::new(|| {
        let mut connection = SqliteConnection::in_memory()?;
        Query::raw(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        )
        .execute(&mut connection)?;
        Query::table("users")
            .insert([("id", Value::U64(7)), ("name", Value::from("Ada"))])
            .execute(&mut connection)?;
        Ok(connection)
    })
}

fn request(path: &str) -> Request {
    Request::new(
        Method::new("POST").unwrap(),
        path,
        Headers::new(),
        Vec::new(),
    )
    .unwrap()
}

fn name_from_row(row: &Row) -> framework::database::Result<String> {
    match row.get("name") {
        Some(Value::Text(name)) => Ok(name.clone()),
        _ => Err(DatabaseError::new(
            ErrorKind::Decode,
            "users.name was not returned as text",
        )),
    }
}

fn current_name(connection: &mut dyn framework::database::Connection) -> Result<String> {
    let row = Query::table("users")
        .where_("id", "=", 7_u64)
        .first(connection)?
        .ok_or_else(|| DatabaseError::new(ErrorKind::Decode, "user 7 is missing"))?;
    Ok(name_from_row(&row)?)
}

fn commit_handler(request: Request) -> Result<Response> {
    let name = request.transaction(TransactionOptions::default(), |connection| {
        Query::table("users")
            .where_("id", "=", 7_u64)
            .update([("name", Value::from("Grace"))])
            .execute(connection)?;
        current_name(connection)
    })?;

    Ok(Response::text(name))
}

fn rollback_handler(request: Request) -> Result<Response> {
    let result: Result<()> = request.transaction(TransactionOptions::default(), |connection| {
        Query::table("users")
            .where_("id", "=", 7_u64)
            .update([("name", Value::from("Grace"))])
            .execute(connection)?;
        Err(ConfigError::new("sqlite-transaction-test", "force rollback").into())
    });
    assert!(matches!(result, Err(Error::Configuration(_))));

    let mut connection = request.connection()?;
    Ok(Response::text(current_name(&mut *connection)?))
}

fn read_only_handler(request: Request) -> Result<Response> {
    request.transaction(TransactionOptions { read_only: true }, |_connection| Ok(()))?;
    Ok(Response::text("unexpected"))
}

#[test]
fn sqlite_request_transaction_commits_real_driver_work() {
    let mut app = App::new();
    app.database(database()).unwrap();
    app.route().post("/commit", commit_handler).unwrap();

    let response = app.handle(request("/commit")).unwrap();
    assert_eq!(response.body(), b"Grace");
}

#[test]
fn sqlite_request_transaction_rolls_back_real_driver_work() {
    let mut app = App::new();
    app.database(database()).unwrap();
    app.route().post("/rollback", rollback_handler).unwrap();

    let response = app.handle(request("/rollback")).unwrap();
    assert_eq!(response.body(), b"Ada");
}

#[test]
fn sqlite_request_transaction_reports_unsupported_read_only_mode() {
    let mut app = App::new();
    app.database(database()).unwrap();
    app.route().post("/read-only", read_only_handler).unwrap();

    let error = app.handle(request("/read-only")).unwrap_err();
    match error {
        Error::Database(error) => assert!(matches!(error.kind(), ErrorKind::Unsupported { .. })),
        other => panic!("expected unsupported database error, got {other}"),
    }
}
