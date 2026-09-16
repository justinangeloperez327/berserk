#![cfg(feature = "database")]

use framework::{
    database::{
        Capabilities, Connection, Database, DatabaseError, Driver, ErrorKind, Execution, Query, Row,
        Statement, Transaction, TransactionOptions, Value,
    },
    App, ConfigError, Error, Headers, Method, Request, Response, Result,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

#[derive(Default)]
struct TransactionState {
    commits: AtomicUsize,
    rollbacks: AtomicUsize,
    executed: Mutex<Vec<Statement>>,
    options: Mutex<Vec<TransactionOptions>>,
}

struct FakeConnection {
    state: Arc<TransactionState>,
}

impl Connection for FakeConnection {
    fn driver(&self) -> Driver {
        Driver::Sqlite
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
    }

    fn execute(&mut self, _statement: &Statement) -> framework::database::Result<Execution> {
        panic!("request transaction writes must use the transaction")
    }

    fn query(&mut self, _statement: &Statement) -> framework::database::Result<Vec<Row>> {
        panic!("request transaction queries must use the transaction")
    }

    fn begin(
        &mut self,
        options: TransactionOptions,
    ) -> framework::database::Result<Box<dyn Transaction + '_>> {
        self.state.options.lock().unwrap().push(options);
        Ok(Box::new(FakeTransaction {
            state: Arc::clone(&self.state),
        }))
    }

    fn ping(&mut self) -> framework::database::Result<()> {
        Ok(())
    }
}

struct FakeTransaction {
    state: Arc<TransactionState>,
}

impl Transaction for FakeTransaction {
    fn execute(&mut self, statement: &Statement) -> framework::database::Result<Execution> {
        self.state.executed.lock().unwrap().push(statement.clone());
        Ok(Execution {
            affected_rows: 1,
            last_insert_id: None,
        })
    }

    fn query(&mut self, _statement: &Statement) -> framework::database::Result<Vec<Row>> {
        Ok(Vec::new())
    }

    fn commit(self: Box<Self>) -> framework::database::Result<()> {
        self.state.commits.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn rollback(self: Box<Self>) -> framework::database::Result<()> {
        self.state.rollbacks.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
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

fn commit_handler(request: Request) -> Result<Response> {
    request.transaction(TransactionOptions::default(), |connection| {
        Query::table("users")
            .where_("id", "=", 7_u64)
            .update([("name", Value::from("Grace"))])
            .execute(connection)?;
        Ok(())
    })?;
    Ok(Response::text("committed"))
}

fn rollback_handler(request: Request) -> Result<Response> {
    request.transaction(TransactionOptions::default(), |_connection| {
        Err(ConfigError::new("transaction-test", "force rollback").into())
    })?;
    Ok(Response::text("unreachable"))
}

fn read_only_handler(request: Request) -> Result<Response> {
    request.transaction(TransactionOptions { read_only: true }, |_connection| Ok(()))?;
    Ok(Response::text("read only"))
}

fn nested_begin_handler(request: Request) -> Result<Response> {
    request.transaction(TransactionOptions::default(), |connection| {
        let error = connection.begin(TransactionOptions::default()).unwrap_err();
        assert!(matches!(error.kind(), ErrorKind::Transaction));
        Ok(())
    })?;
    Ok(Response::text("nested rejected"))
}

#[test]
fn request_transaction_commits_bound_sql_on_success() {
    let state = Arc::new(TransactionState::default());
    let acquisitions = Arc::new(AtomicUsize::new(0));
    let factory_state = Arc::clone(&state);
    let factory_acquisitions = Arc::clone(&acquisitions);
    let mut app = App::new();
    app.database(Database::new(move || {
        factory_acquisitions.fetch_add(1, Ordering::SeqCst);
        Ok(FakeConnection {
            state: Arc::clone(&factory_state),
        })
    }))
    .unwrap();
    app.route().post("/commit", commit_handler).unwrap();

    let response = app.handle(request("/commit")).unwrap();
    assert_eq!(response.body(), b"committed");
    assert_eq!(acquisitions.load(Ordering::SeqCst), 1);
    assert_eq!(state.commits.load(Ordering::SeqCst), 1);
    assert_eq!(state.rollbacks.load(Ordering::SeqCst), 0);

    let executed = state.executed.lock().unwrap();
    assert_eq!(executed.len(), 1);
    assert_eq!(
        executed[0].sql(),
        "UPDATE \"users\" SET \"name\" = ? WHERE \"id\" = ?"
    );
    assert_eq!(
        executed[0].bindings(),
        &[Value::Text("Grace".into()), Value::U64(7)]
    );
}

#[test]
fn request_transaction_rolls_back_and_preserves_application_error() {
    let state = Arc::new(TransactionState::default());
    let factory_state = Arc::clone(&state);
    let mut app = App::new();
    app.database(Database::new(move || {
        Ok(FakeConnection {
            state: Arc::clone(&factory_state),
        })
    }))
    .unwrap();
    app.route().post("/rollback", rollback_handler).unwrap();

    let error = app.handle(request("/rollback")).unwrap_err();
    assert!(matches!(error, Error::Configuration(_)));
    assert_eq!(state.commits.load(Ordering::SeqCst), 0);
    assert_eq!(state.rollbacks.load(Ordering::SeqCst), 1);
}

#[test]
fn request_transaction_forwards_options_and_rejects_nested_begin() {
    let state = Arc::new(TransactionState::default());
    let factory_state = Arc::clone(&state);
    let mut app = App::new();
    app.database(Database::new(move || {
        Ok(FakeConnection {
            state: Arc::clone(&factory_state),
        })
    }))
    .unwrap();
    {
        let mut route = app.route();
        route.post("/read-only", read_only_handler).unwrap();
        route.post("/nested", nested_begin_handler).unwrap();
    }

    assert_eq!(app.handle(request("/read-only")).unwrap().body(), b"read only");
    assert_eq!(
        app.handle(request("/nested")).unwrap().body(),
        b"nested rejected"
    );

    let options = state.options.lock().unwrap();
    assert_eq!(options.len(), 2);
    assert!(options[0].read_only);
    assert!(!options[1].read_only);
    assert_eq!(state.commits.load(Ordering::SeqCst), 2);
    assert_eq!(state.rollbacks.load(Ordering::SeqCst), 0);
}
