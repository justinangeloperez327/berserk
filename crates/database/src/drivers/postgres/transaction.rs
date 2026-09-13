use crate::{Execution, Result, Row, Statement, Transaction};

pub(super) struct PostgresTransaction<'a> {
    inner: postgres::Transaction<'a>,
}

impl<'a> PostgresTransaction<'a> {
    pub(super) fn new(inner: postgres::Transaction<'a>) -> Self {
        Self { inner }
    }
}

impl Transaction for PostgresTransaction<'_> {
    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        super::convert::execute(&mut self.inner, statement).map(|affected_rows| Execution {
            affected_rows,
            last_insert_id: None,
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
