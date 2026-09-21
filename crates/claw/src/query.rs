use crate::{Collection, Model, Page};
use berserk_database::{
    Connection, DatabaseError, Direction, Driver, ErrorKind, Execution, Query, Result, Statement,
    Value,
};
use std::marker::PhantomData;

/// A query builder whose returned rows are decoded as `M`.
#[derive(Clone, Debug)]
pub struct ModelQuery<M> {
    builder: Query,
    marker: PhantomData<fn() -> M>,
}

impl<M: Model> ModelQuery<M> {
    pub fn where_(self, column: impl Into<String>, value: impl Into<Value>) -> Self {
        self.where_op(column, "=", value)
    }
    pub fn or_where(self, column: impl Into<String>, value: impl Into<Value>) -> Self {
        self.or_where_op(column, "=", value)
    }
    pub fn get(&self) -> Result<Collection<M>> {
        berserk_database::scope::with_connection(|c| self.get_on(c))
    }
    pub fn first(self) -> Result<Option<M>> {
        berserk_database::scope::with_connection(|c| self.first_on(c))
    }
    pub fn first_or_fail(self) -> Result<M> {
        self.first()?.ok_or_else(crate::writes::not_found)
    }
    pub fn count(&self) -> Result<u64> {
        berserk_database::scope::with_connection(|c| self.count_on(c))
    }
    pub fn exists(&self) -> Result<bool> {
        berserk_database::scope::with_connection(|c| self.exists_on(c))
    }
    pub fn update(self, input: impl crate::IntoUpdate<M>) -> Result<Execution> {
        let values = crate::writes::allowed_values::<M>(input.into_update()?)?;
        berserk_database::scope::with_connection(|c| self.update_on(c, values))
    }
    pub fn delete(self) -> Result<Execution> {
        berserk_database::scope::with_connection(|c| self.delete_on(c))
    }
    pub fn paginate(self, per_page: u64) -> Result<Page<M>> {
        let page = berserk_database::scope::current_page()?;
        berserk_database::scope::with_connection(|c| self.paginate_on(c, page, per_page))
    }
    pub fn with<E, Kind>(
        self,
        relations: E,
    ) -> <E as crate::IntoEager<M, Kind>>::Query
    where
        E: crate::IntoEager<M, Kind>,
    {
        relations.into_eager(self)
    }

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

    pub fn where_op(
        mut self,
        column: impl Into<String>,
        operator: impl Into<String>,
        value: impl Into<Value>,
    ) -> Self {
        self.builder = self.builder.where_(column, operator, value);
        self
    }

    pub fn or_where_op(
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

    pub fn or_where_in<I, V>(mut self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.builder = self.builder.or_where_in(column, values);
        self
    }

    pub fn or_where_not_in<I, V>(mut self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.builder = self.builder.or_where_not_in(column, values);
        self
    }

    pub fn where_between(
        mut self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.builder = self.builder.where_between(column, lower, upper);
        self
    }

    pub fn or_where_between(
        mut self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.builder = self.builder.or_where_between(column, lower, upper);
        self
    }

    pub fn where_not_between(
        mut self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.builder = self.builder.where_not_between(column, lower, upper);
        self
    }

    pub fn or_where_not_between(
        mut self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.builder = self.builder.or_where_not_between(column, lower, upper);
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

    pub fn or_where_null(mut self, column: impl Into<String>) -> Self {
        self.builder = self.builder.or_where_null(column);
        self
    }

    pub fn or_where_not_null(mut self, column: impl Into<String>) -> Self {
        self.builder = self.builder.or_where_not_null(column);
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

    pub fn get_on(&self, connection: &mut dyn Connection) -> Result<Collection<M>> {
        self.builder
            .get(connection)?
            .iter()
            .map(M::from_row)
            .collect::<Result<Vec<_>>>()
            .map(Collection::from)
    }

    pub fn first_on(self, connection: &mut dyn Connection) -> Result<Option<M>> {
        self.builder
            .first(connection)?
            .as_ref()
            .map(M::from_row)
            .transpose()
    }

    pub fn count_on(&self, connection: &mut dyn Connection) -> Result<u64> {
        self.builder.count(connection)
    }

    pub fn exists_on(&self, connection: &mut dyn Connection) -> Result<bool> {
        Ok(!self.builder.clone().limit(1).get(connection)?.is_empty())
    }

    pub fn update_on<I, S, V>(self, connection: &mut dyn Connection, values: I) -> Result<Execution>
    where
        I: IntoIterator<Item = (S, V)>,
        S: Into<String>,
        V: Into<Value>,
    {
        self.builder.update(values).execute(connection)
    }

    pub fn delete_on(self, connection: &mut dyn Connection) -> Result<Execution> {
        self.builder.delete().execute(connection)
    }

    pub fn paginate_on(
        self,
        connection: &mut dyn Connection,
        page: u64,
        per_page: u64,
    ) -> Result<Page<M>> {
        Page::<M>::validate(page, per_page)?;
        let offset = (page - 1)
            .checked_mul(per_page)
            .filter(|offset| *offset <= i64::MAX as u64)
            .ok_or_else(|| {
                DatabaseError::new(ErrorKind::InvalidInput, "pagination offset overflow")
            })?;
        let total = self.builder.count(connection)?;
        let items = Self {
            builder: self.builder.limit(per_page).offset(offset),
            marker: PhantomData,
        }
        .get_on(connection)?;
        Page::new(items, page, per_page, total)
    }
}

impl<M: Model> Default for ModelQuery<M> {
    fn default() -> Self {
        Self::new()
    }
}
