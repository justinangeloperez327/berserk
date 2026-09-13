use crate::{
    Capabilities, Capability, Connection, Driver, Execution, Result, Row, Statement, Transaction,
    TransactionOptions,
};
use postgres::{Client, NoTls};

pub struct PostgresConnection {
    client: Client,
}

impl PostgresConnection {
    /// Connects without TLS. Use `from_client` with a securely configured
    /// `postgres::Client` when transport encryption is required.
    pub fn connect_no_tls(parameters: &str) -> Result<Self> {
        Client::connect(parameters, NoTls)
            .map(Self::from_client)
            .map_err(super::error::map_error)
    }

    pub fn from_client(client: Client) -> Self {
        Self { client }
    }
    pub fn client(&self) -> &Client {
        &self.client
    }
    pub fn client_mut(&mut self) -> &mut Client {
        &mut self.client
    }
}

impl Connection for PostgresConnection {
    fn driver(&self) -> Driver {
        Driver::Postgres
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
            .with(Capability::Returning)
            .with(Capability::Savepoints)
            .with(Capability::TransactionalDdl)
            .with(Capability::AdvisoryLocks)
            .with(Capability::ReadOnlyTransactions)
    }

    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        super::convert::execute(&mut self.client, statement).map(|affected_rows| Execution {
            affected_rows,
            last_insert_id: None,
        })
    }

    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        super::convert::query(&mut self.client, statement)
    }

    fn begin(&mut self, options: TransactionOptions) -> Result<Box<dyn Transaction + '_>> {
        let transaction = self
            .client
            .build_transaction()
            .read_only(options.read_only)
            .start()
            .map_err(super::error::map_error)?;
        Ok(Box::new(super::transaction::PostgresTransaction::new(
            transaction,
        )))
    }

    fn ping(&mut self) -> Result<()> {
        self.client
            .check_connection()
            .map_err(super::error::map_error)
    }
}
