use crate::{Execution, Result, Row, Statement, Transaction};

pub(super) struct MySqlTransaction<'a> {
    inner: mysql::Transaction<'a>,
}
impl<'a> MySqlTransaction<'a> {
    pub(super) fn new(inner: mysql::Transaction<'a>) -> Self {
        Self { inner }
    }
}

impl Transaction for MySqlTransaction<'_> {
    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        super::convert::execute(&mut self.inner, statement)?;
        Ok(Execution {
            affected_rows: self.inner.affected_rows(),
            last_insert_id: self.inner.last_insert_id(),
        })
    }
    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        super::convert::query(&mut self.inner, statement)
    }
    fn commit(self: Box<Self>) -> Result<()> {
        let Self { inner } = *self;
        inner.commit().map_err(super::error::map_error)
    }
    fn rollback(self: Box<Self>) -> Result<()> {
        let Self { inner } = *self;
        inner.rollback().map_err(super::error::map_error)
    }
}
