use crate::{Model, Page};
use framework_database::{
    Connection, DatabaseError, Direction, Driver, ErrorKind, Query, Result, Statement, Value,
};
use std::marker::PhantomData;

/// A query builder whose returned rows are decoded as `M`.
#[derive(Clone, Debug)]
pub struct ModelQuery<M> {
    builder: Query,
    marker: PhantomData<fn() -> M>,
}

impl<M: Model> ModelQuery<M> {
    pub fn new() -> Self {
        Self {
            builder: Query::table(M::TABLE),
            marker: PhantomData,
        }
    }

    pub fn select<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.builder = self.builder.select(columns);
        self
    }

    pub fn where_(
        mut self,
        column: impl Into<String>,
        operator: impl Into<String>,
        value: impl Into<Value>,
    ) -> Self {
        self.builder = self.builder.where_(column, operator, value);
        self
    }

    pub fn or_where(
        mut self,
        column: impl Into<String>,
        operator: impl Into<String>,
        value: impl Into<Value>,
    ) -> Self {
        self.builder = self.builder.or_where(column, operator, value);
        self
    }

    pub fn where_in<I, V>(mut self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.builder = self.builder.where_in(column, values);
        self
    }

    pub fn where_not_in<I, V>(mut self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.builder = self.builder.where_not_in(column, values);
        self
    }

    pub fn where_null(mut self, column: impl Into<String>) -> Self {
        self.builder = self.builder.where_null(column);
        self
    }

    pub fn where_not_null(mut self, column: impl Into<String>) -> Self {
        self.builder = self.builder.where_not_null(column);
        self
    }

    pub fn order_by(mut self, column: impl Into<String>, direction: Direction) -> Self {
        self.builder = self.builder.order_by(column, direction);
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.builder = self.builder.limit(limit);
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.builder = self.builder.offset(offset);
        self
    }

    pub fn scope(mut self, scope: impl FnOnce(Query) -> Query) -> Self {
        self.builder = scope(self.builder);
        self
    }

    pub fn builder(&self) -> &Query {
        &self.builder
    }

    pub fn into_builder(self) -> Query {
        self.builder
    }

    pub fn to_statement(&self, driver: Driver) -> Result<Statement> {
        self.builder.to_statement(driver)
    }

    pub fn get(&self, connection: &mut dyn Connection) -> Result<Vec<M>> {
        self.builder
            .get(connection)?
            .iter()
            .map(M::from_row)
            .collect()
    }

    pub fn first(self, connection: &mut dyn Connection) -> Result<Option<M>> {
        self.builder
            .first(connection)?
            .as_ref()
            .map(M::from_row)
            .transpose()
    }

    pub fn paginate(
        self,
        connection: &mut dyn Connection,
        page: u64,
        per_page: u64,
    ) -> Result<Page<M>> {
        Page::<M>::validate(page, per_page)?;
        let offset = (page - 1).checked_mul(per_page).ok_or_else(|| {
            DatabaseError::new(ErrorKind::Query, "pagination offset overflow")
        })?;
        let total = self.builder.count(connection)?;
        let items = Self {
            builder: self.builder.limit(per_page).offset(offset),
            marker: PhantomData,
        }
        .get(connection)?;
        Page::new(items, page, per_page, total)
    }
}

impl<M: Model> Default for ModelQuery<M> {
    fn default() -> Self {
        Self::new()
    }
}
