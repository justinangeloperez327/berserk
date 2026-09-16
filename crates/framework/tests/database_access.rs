#![cfg(feature = "database")]

use framework::{
    database::{
        Capabilities, Connection, Database, DatabaseError, Driver, ErrorKind, Execution, Row,
        Statement, Transaction, TransactionOptions,
    },
    App, Error, Headers, Method, Request, Response, Result,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct FakeConnection {
    pings: Arc<AtomicUsize>,
}

impl Connection for FakeConnection {
    fn driver(&self) -> Driver {
        Driver::Sqlite
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
    }

    fn execute(&mut self, _statement: &Statement) -> framework::database::Result<Execution> {
        Ok(Execution {
            affected_rows: 0,
            last_insert_id: None,
        })
    }

    fn query(&mut self, _statement: &Statement) -> framework::database::Result<Vec<Row>> {
        Ok(Vec::new())
    }

    fn begin(
        &mut self,
        _options: TransactionOptions,
    ) -> framework::database::Result<Box<dyn Transaction + '_>> {
        Err(DatabaseError::new(
            ErrorKind::Transaction,
            "transactions are not used by this test",
        ))
    }

    fn ping(&mut self) -> framework::database::Result<()> {
        self.pings.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn request(path: &str) -> Request {
    Request::new(
        Method::new("GET").unwrap(),
        path,
        Headers::new(),
        Vec::new(),
    )
    .unwrap()
}

fn database_handler(request: Request) -> Result<Response> {
    let _database = request.database()?;
    {
        let mut connection = request.connection()?;
        connection.ping()?;
    }
    {
        let mut connection = request.connection()?;
        connection.ping()?;
    }
    Ok(Response::text("database ready"))
}

fn overlapping_borrow_handler(request: Request) -> Result<Response> {
    let _connection = request.connection()?;
    match request.connection() {
        Err(Error::Database(error)) if matches!(error.kind(), ErrorKind::Connection) => {
            Ok(Response::text("borrow rejected"))
        }
        Err(error) => Err(error),
        Ok(_) => panic!("overlapping request connection borrow unexpectedly succeeded"),
    }
}

#[test]
fn app_and_request_database_helpers_share_one_request_connection() {
    let acquisitions = Arc::new(AtomicUsize::new(0));
    let pings = Arc::new(AtomicUsize::new(0));
    let factory_acquisitions = Arc::clone(&acquisitions);
    let factory_pings = Arc::clone(&pings);

    let mut app = App::new();
    app.database(Database::new(move || {
        factory_acquisitions.fetch_add(1, Ordering::SeqCst);
        Ok(FakeConnection {
            pings: Arc::clone(&factory_pings),
        })
    }))
    .unwrap();
    app.route().get("/database", database_handler).unwrap();

    let response = app.handle(request("/database")).unwrap();
    assert_eq!(response.body(), b"database ready");
    assert_eq!(acquisitions.load(Ordering::SeqCst), 1);
    assert_eq!(pings.load(Ordering::SeqCst), 2);
}

#[test]
fn overlapping_request_connection_borrow_returns_an_error() {
    let pings = Arc::new(AtomicUsize::new(0));
    let factory_pings = Arc::clone(&pings);
    let mut app = App::new();
    app.database(Database::new(move || {
        Ok(FakeConnection {
            pings: Arc::clone(&factory_pings),
        })
    }))
    .unwrap();
    app.route()
        .get("/database/borrow", overlapping_borrow_handler)
        .unwrap();

    let response = app.handle(request("/database/borrow")).unwrap();
    assert_eq!(response.body(), b"borrow rejected");
}

#[test]
fn request_database_helpers_report_missing_configuration() {
    let mut app = App::new();
    app.route().get("/database", database_handler).unwrap();

    let error = app.handle(request("/database")).unwrap_err();
    assert!(matches!(error, Error::Configuration(_)));
}
