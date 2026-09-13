use crate::{
    Capabilities, Capability, Connection, DatabaseError, Driver, ErrorKind, Execution, Result, Row,
    Statement, Transaction, TransactionOptions,
};
use std::path::Path;

pub struct SqliteConnection {
    connection: rusqlite::Connection,
}

impl SqliteConnection {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        rusqlite::Connection::open(path)
            .map(Self::from_connection)
            .map_err(super::error::map_error)
    }
    pub fn in_memory() -> Result<Self> {
        rusqlite::Connection::open_in_memory()
            .map(Self::from_connection)
            .map_err(super::error::map_error)
    }
    pub fn from_connection(connection: rusqlite::Connection) -> Self {
        Self { connection }
    }
    pub fn connection(&self) -> &rusqlite::Connection {
        &self.connection
    }
    pub fn connection_mut(&mut self) -> &mut rusqlite::Connection {
        &mut self.connection
    }
}

impl Connection for SqliteConnection {
    fn driver(&self) -> Driver {
        Driver::Sqlite
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
            .with(Capability::Returning)
            .with(Capability::Savepoints)
            .with(Capability::TransactionalDdl)
    }
    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        super::convert::execute(&self.connection, statement).map(|affected_rows| Execution {
            affected_rows,
            last_insert_id: None,
        })
    }
    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        super::convert::query(&self.connection, statement)
    }
    fn begin(&mut self, options: TransactionOptions) -> Result<Box<dyn Transaction + '_>> {
        if options.read_only {
            return Err(DatabaseError::new(
                ErrorKind::Unsupported {
                    driver: Driver::Sqlite,
                    capability: Capability::ReadOnlyTransactions,
                },
                "SQLite does not provide a transaction-scoped read-only mode",
            ));
        }
        let transaction = self
            .connection
            .transaction()
            .map_err(super::error::map_error)?;
        Ok(Box::new(super::transaction::SqliteTransaction::new(
            transaction,
        )))
    }
    fn ping(&mut self) -> Result<()> {
        self.connection
            .query_row("select 1", [], |_| Ok(()))
            .map_err(super::error::map_error)
    }
}
