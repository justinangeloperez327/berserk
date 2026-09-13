use crate::{
    Capabilities, Driver, Execution, Result, Row, Statement, Transaction, TransactionOptions,
};

pub trait Connection: Send {
    fn driver(&self) -> Driver;
    fn capabilities(&self) -> Capabilities;
    fn execute(&mut self, statement: &Statement) -> Result<Execution>;
    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>>;
    fn begin(&mut self, options: TransactionOptions) -> Result<Box<dyn Transaction + '_>>;
    fn ping(&mut self) -> Result<()>;
}
