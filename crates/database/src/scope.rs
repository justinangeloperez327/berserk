//! Lexically scoped database access. A scope is confined to its calling worker.
use crate::{
    Capabilities, Connection, Database, DatabaseError, Driver, ErrorKind, Execution, Result, Row,
    Statement, Transaction, TransactionOptions,
};
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    sync::{Mutex, MutexGuard},
};

trait Access {
    fn execute(&self, operation: &mut dyn FnMut(&mut dyn Connection) -> Result<()>) -> Result<()>;
    fn page(&self) -> Result<u64>;
}
scoped_tls_hkt::scoped_thread_local!(static CURRENT: for<'a> &'a (dyn Access + 'a));

/// A lazy connection shared by the operations in one request or job.
/// Nested scopes restore their parent on return and unwind. Spawned tasks do not inherit a scope.
pub struct DatabaseScope {
    database: Option<Database>,
    connection: Mutex<Option<Box<dyn Connection + Send>>>,
    page: Result<u64>,
}
impl DatabaseScope {
    pub fn new(database: Database) -> Self {
        Self::optional(Some(database), Ok(1))
    }
    pub fn optional(database: Option<Database>, page: Result<u64>) -> Self {
        Self {
            database,
            connection: Mutex::new(None),
            page,
        }
    }
    pub fn run<T>(&self, operation: impl FnOnce() -> T) -> T {
        CURRENT.set(self, operation)
    }
    pub fn connection(&self) -> Result<ScopedConnection<'_>> {
        let mut inner = self.connection.try_lock().map_err(|_| busy())?;
        if inner.is_none() {
            let database = self.database.as_ref().ok_or_else(|| {
                DatabaseError::new(
                    ErrorKind::Configuration,
                    "no database is configured in the current scope",
                )
            })?;
            *inner = Some(database.acquire()?);
        }
        Ok(ScopedConnection { inner })
    }
}
impl Access for DatabaseScope {
    fn execute(&self, operation: &mut dyn FnMut(&mut dyn Connection) -> Result<()>) -> Result<()> {
        operation(&mut *self.connection()?)
    }
    fn page(&self) -> Result<u64> {
        self.page.clone()
    }
}
/// A checked exclusive connection borrow. Drop it before using scoped ORM methods.
pub struct ScopedConnection<'a> {
    inner: MutexGuard<'a, Option<Box<dyn Connection + Send>>>,
}
impl Deref for ScopedConnection<'_> {
    type Target = dyn Connection;
    fn deref(&self) -> &Self::Target {
        self.inner.as_deref().expect("initialized connection")
    }
}
impl DerefMut for ScopedConnection<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_deref_mut().expect("initialized connection")
    }
}
fn busy() -> DatabaseError {
    DatabaseError::new(
        ErrorKind::Connection,
        "scoped connection is already borrowed or poisoned",
    )
}

pub fn with_connection<T, E: From<DatabaseError>>(
    operation: impl FnOnce(&mut dyn Connection) -> std::result::Result<T, E>,
) -> std::result::Result<T, E> {
    if !CURRENT.is_set() {
        return Err(
            DatabaseError::new(ErrorKind::Configuration, "no database scope is active").into(),
        );
    }
    let mut operation = Some(operation);
    let mut result = None;
    CURRENT.with(|scope| {
        scope.execute(&mut |connection| {
            result = Some(operation.take().expect("called once")(connection));
            Ok(())
        })
    })?;
    result.expect("scope executes callback once")
}
pub fn current_page() -> Result<u64> {
    if CURRENT.is_set() {
        CURRENT.with(|scope| scope.page())
    } else {
        Ok(1)
    }
}

/// Install an explicit connection for a synchronous operation, restoring the enclosing scope.
pub fn with_scoped_connection<T>(
    connection: &mut dyn Connection,
    operation: impl FnOnce() -> T,
) -> T {
    struct Borrowed<'a> {
        connection: RefCell<&'a mut dyn Connection>,
        page: Result<u64>,
    }
    impl Access for Borrowed<'_> {
        fn execute(
            &self,
            operation: &mut dyn FnMut(&mut dyn Connection) -> Result<()>,
        ) -> Result<()> {
            operation(&mut **self.connection.try_borrow_mut().map_err(|_| busy())?)
        }
        fn page(&self) -> Result<u64> {
            self.page.clone()
        }
    }
    let scope = Borrowed {
        connection: RefCell::new(connection),
        page: current_page(),
    };
    CURRENT.set(&scope, operation)
}

pub fn transaction<T, E: From<DatabaseError>>(
    options: TransactionOptions,
    operation: impl FnOnce() -> std::result::Result<T, E>,
) -> std::result::Result<T, E> {
    with_connection(|connection| {
        let driver = connection.driver();
        let capabilities = connection.capabilities();
        let mut transaction = connection.begin(options)?;
        let result = with_scoped_connection(
            &mut TransactionConnection {
                transaction: &mut *transaction,
                driver,
                capabilities,
            },
            operation,
        );
        match result {
            Ok(value) => {
                transaction.commit()?;
                Ok(value)
            }
            Err(error) => {
                transaction.rollback()?;
                Err(error)
            }
        }
    })
}
struct TransactionConnection<'a> {
    transaction: &'a mut dyn Transaction,
    driver: Driver,
    capabilities: Capabilities,
}
impl Connection for TransactionConnection<'_> {
    fn driver(&self) -> Driver {
        self.driver
    }
    fn capabilities(&self) -> Capabilities {
        self.capabilities
    }
    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        self.transaction.execute(statement)
    }
    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        self.transaction.query(statement)
    }
    fn begin(&mut self, _: TransactionOptions) -> Result<Box<dyn Transaction + '_>> {
        Err(DatabaseError::new(
            ErrorKind::Transaction,
            "nested transactions are not supported",
        ))
    }
    fn ping(&mut self) -> Result<()> {
        Err(DatabaseError::new(
            ErrorKind::Transaction,
            "ping is not available inside a transaction",
        ))
    }
}
