use crate::{Connection, Driver, Execution, Result, Row, Statement};

/// Minimal database execution boundary shared by connections and transaction adapters.
///
/// Query builders and ORM terminals depend on this contract rather than the full
/// connection lifecycle. Normal connections implement it automatically.
pub trait Executor {
    fn driver(&self) -> Driver;
    fn execute(&mut self, statement: &Statement) -> Result<Execution>;
    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>>;
}

impl<T> Executor for T
where
    T: Connection + ?Sized,
{
    fn driver(&self) -> Driver {
        Connection::driver(self)
    }

    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        Connection::execute(self, statement)
    }

    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        Connection::query(self, statement)
    }
}
