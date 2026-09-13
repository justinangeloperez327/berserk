use crate::{Execution, Result, Row, Statement, Transaction};

pub(super) struct SqliteTransaction<'a> {
    inner: rusqlite::Transaction<'a>,
}
impl<'a> SqliteTransaction<'a> {
    pub(super) fn new(inner: rusqlite::Transaction<'a>) -> Self {
        Self { inner }
    }
}

impl Transaction for SqliteTransaction<'_> {
    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        super::convert::execute(&self.inner, statement).map(|affected_rows| Execution {
            affected_rows,
            last_insert_id: None,
        })
    }
    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        super::convert::query(&self.inner, statement)
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
