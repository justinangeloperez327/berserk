use crate::{Execution, Result, Row, Statement};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TransactionOptions {
    pub read_only: bool,
}

pub trait Transaction {
    fn execute(&mut self, statement: &Statement) -> Result<Execution>;
    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>>;
    fn commit(self: Box<Self>) -> Result<()>;
    fn rollback(self: Box<Self>) -> Result<()>;
}
