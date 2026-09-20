use crate::{
    Capabilities, Capability, Connection, Driver, Execution, Result, Row, Statement, Transaction,
    TransactionOptions,
};
use mysql::{AccessMode, Conn, Opts, TxOpts};

pub struct MySqlConnection {
    connection: Conn,
}

impl MySqlConnection {
    pub fn connect(url: &str) -> Result<Self> {
        let options = Opts::from_url(url).map_err(|error| super::error::map_error(error.into()))?;
        Conn::new(options)
            .map(Self::from_connection)
            .map_err(super::error::map_error)
    }
    pub fn from_connection(connection: Conn) -> Self {
        Self { connection }
    }
    pub fn connection(&self) -> &Conn {
        &self.connection
    }
    pub fn connection_mut(&mut self) -> &mut Conn {
        &mut self.connection
    }
}

impl Connection for MySqlConnection {
    fn driver(&self) -> Driver {
        Driver::MySql
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
            .with(Capability::Savepoints)
            .with(Capability::AdvisoryLocks)
            .with(Capability::ReadOnlyTransactions)
    }
    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        super::convert::execute(&mut self.connection, statement)?;
        Ok(Execution {
            affected_rows: self.connection.affected_rows(),
            last_insert_id: match self.connection.last_insert_id() {
                0 => None,
                value => Some(value),
            },
        })
    }
    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        super::convert::query(&mut self.connection, statement)
    }
    fn begin(&mut self, options: TransactionOptions) -> Result<Box<dyn Transaction + '_>> {
        let mode = if options.read_only {
            Some(AccessMode::ReadOnly)
        } else {
            None
        };
        let options = TxOpts::default().set_access_mode(mode);
        let transaction = self
            .connection
            .start_transaction(options)
            .map_err(super::error::map_error)?;
        Ok(Box::new(super::transaction::MySqlTransaction::new(
            transaction,
        )))
    }
    fn ping(&mut self) -> Result<()> {
        self.connection.ping().map_err(super::error::map_error)
    }
}
