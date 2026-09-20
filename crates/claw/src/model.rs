use crate::ModelQuery;
use berserk_database::{Connection, Direction, Execution, Query, Result, Row, Value};

/// A typed database record managed by Claw ORM.
pub trait Model: Sized {
    const TABLE: &'static str;
    const PRIMARY_KEY: &'static str = "id";
    /// Columns accepted by typed create/update. Empty is deny-all.
    const FILLABLE: &'static [&'static str] = &[];

    fn from_row(row: &Row) -> Result<Self>;
    fn key(&self) -> Value;

    /// Convert a raw route parameter into this model's lookup key.
    ///
    /// Numeric `u64` keys are the default. Models using strings, UUIDs, or
    /// another key representation can override this method without changing
    /// the router or controller signature.
    fn parse_route_key(value: &str) -> Option<Value> {
        value.parse::<u64>().ok().map(Value::from)
    }

    fn where_(column: impl Into<String>, value: impl Into<Value>) -> ModelQuery<Self> {
        Self::query().where_(column, value)
    }
    fn or_where(column: impl Into<String>, value: impl Into<Value>) -> ModelQuery<Self> {
        Self::query().or_where(column, value)
    }
    fn or_where_op(
        column: impl Into<String>,
        operator: impl Into<String>,
        value: impl Into<Value>,
    ) -> ModelQuery<Self> {
        Self::query().or_where_op(column, operator, value)
    }
    fn all() -> Result<Vec<Self>> {
        Self::query().get()
    }
    fn first() -> Result<Option<Self>> {
        Self::query().first()
    }
    fn count() -> Result<u64> {
        Self::query().count()
    }
    fn exists() -> Result<bool> {
        Self::query().exists()
    }
    fn find(key: impl Into<Value>) -> Result<Option<Self>> {
        berserk_database::scope::with_connection(|c| Self::find_on(c, key))
    }
    fn find_or_fail(key: impl Into<Value>) -> Result<Self> {
        Self::find(key)?.ok_or_else(crate::writes::not_found)
    }
    fn find_many<I, V>(keys: I) -> Result<Vec<Self>>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        Self::where_in(Self::PRIMARY_KEY, keys).get()
    }
    fn fresh(&self) -> Result<Option<Self>> {
        Self::find(self.key())
    }
    fn refresh(&mut self) -> Result<bool> {
        berserk_database::scope::with_connection(|c| self.refresh_on(c))
    }
    fn create(input: impl crate::IntoInsert<Self>) -> Result<Self> {
        let values = crate::writes::allowed_values::<Self>(input.into_insert()?)?;
        berserk_database::scope::with_connection(|c| crate::writes::insert_model(c, values))
    }
    fn update(&mut self, input: impl crate::IntoUpdate<Self>) -> Result<()> {
        let values = crate::writes::allowed_values::<Self>(input.into_update()?)?;
        berserk_database::scope::with_connection(|c| {
            self.update_on(c, values)?;
            if self.refresh_on(c)? {
                Ok(())
            } else {
                Err(crate::writes::not_found())
            }
        })
    }
    fn destroy(key: impl Into<Value>) -> Result<Execution> {
        berserk_database::scope::with_connection(|c| Self::destroy_on(c, key))
    }
    fn delete(&self) -> Result<Execution> {
        berserk_database::scope::with_connection(|c| self.delete_on(c))
    }

    fn query() -> ModelQuery<Self> {
        ModelQuery::new()
    }

    fn select<I, S>(columns: I) -> ModelQuery<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::query().select(columns)
    }

    fn where_op(
        column: impl Into<String>,
        operator: impl Into<String>,
        value: impl Into<Value>,
    ) -> ModelQuery<Self> {
        Self::query().where_op(column, operator, value)
    }

    fn where_in<I, V>(column: impl Into<String>, values: I) -> ModelQuery<Self>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        Self::query().where_in(column, values)
    }

    fn where_not_in<I, V>(column: impl Into<String>, values: I) -> ModelQuery<Self>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        Self::query().where_not_in(column, values)
    }

    fn or_where_in<I, V>(column: impl Into<String>, values: I) -> ModelQuery<Self>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        Self::query().or_where_in(column, values)
    }

    fn or_where_not_in<I, V>(column: impl Into<String>, values: I) -> ModelQuery<Self>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        Self::query().or_where_not_in(column, values)
    }

    fn where_between(
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> ModelQuery<Self> {
        Self::query().where_between(column, lower, upper)
    }

    fn or_where_between(
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> ModelQuery<Self> {
        Self::query().or_where_between(column, lower, upper)
    }

    fn where_not_between(
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> ModelQuery<Self> {
        Self::query().where_not_between(column, lower, upper)
    }

    fn or_where_not_between(
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> ModelQuery<Self> {
        Self::query().or_where_not_between(column, lower, upper)
    }

    fn where_null(column: impl Into<String>) -> ModelQuery<Self> {
        Self::query().where_null(column)
    }

    fn where_not_null(column: impl Into<String>) -> ModelQuery<Self> {
        Self::query().where_not_null(column)
    }

    fn or_where_null(column: impl Into<String>) -> ModelQuery<Self> {
        Self::query().or_where_null(column)
    }

    fn or_where_not_null(column: impl Into<String>) -> ModelQuery<Self> {
        Self::query().or_where_not_null(column)
    }

    fn order_by(column: impl Into<String>, direction: Direction) -> ModelQuery<Self> {
        Self::query().order_by(column, direction)
    }

    fn limit(limit: u64) -> ModelQuery<Self> {
        Self::query().limit(limit)
    }

    fn offset(offset: u64) -> ModelQuery<Self> {
        Self::query().offset(offset)
    }

    fn all_on(connection: &mut dyn Connection) -> Result<Vec<Self>> {
        Self::query().get_on(connection)
    }

    fn count_on(connection: &mut dyn Connection) -> Result<u64> {
        Self::query().count_on(connection)
    }

    fn exists_on(connection: &mut dyn Connection) -> Result<bool> {
        Self::query().exists_on(connection)
    }

    fn find_on(connection: &mut dyn Connection, key: impl Into<Value>) -> Result<Option<Self>> {
        Self::where_op(Self::PRIMARY_KEY, "=", key).first_on(connection)
    }

    fn find_many_on<I, V>(connection: &mut dyn Connection, keys: I) -> Result<Vec<Self>>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        Self::where_in(Self::PRIMARY_KEY, keys).get_on(connection)
    }

    /// Fetch a new instance of this model's current database row.
    fn fresh_on(&self, connection: &mut dyn Connection) -> Result<Option<Self>> {
        Self::find_on(connection, self.key())
    }

    /// Replace this model with its current database row.
    ///
    /// Returns `false` when the row no longer exists.
    fn refresh_on(&mut self, connection: &mut dyn Connection) -> Result<bool> {
        match self.fresh_on(connection)? {
            Some(fresh) => {
                *self = fresh;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn create_on<I, S, V>(connection: &mut dyn Connection, values: I) -> Result<Execution>
    where
        I: IntoIterator<Item = (S, V)>,
        S: Into<String>,
        V: Into<Value>,
    {
        Query::table(Self::TABLE).insert(values).execute(connection)
    }

    fn destroy_on(connection: &mut dyn Connection, key: impl Into<Value>) -> Result<Execution> {
        Self::where_op(Self::PRIMARY_KEY, "=", key).delete_on(connection)
    }

    /// Persist explicit values for this model's primary-key row.
    ///
    /// This low-level operation does not reload the in-memory model. The scoped
    /// `update` operation refreshes through `from_row` after writing.
    fn update_on<I, S, V>(&self, connection: &mut dyn Connection, values: I) -> Result<Execution>
    where
        I: IntoIterator<Item = (S, V)>,
        S: Into<String>,
        V: Into<Value>,
    {
        Self::where_op(Self::PRIMARY_KEY, "=", self.key()).update_on(connection, values)
    }

    /// Delete this model's primary-key row.
    fn delete_on(&self, connection: &mut dyn Connection) -> Result<Execution> {
        Self::where_op(Self::PRIMARY_KEY, "=", self.key()).delete_on(connection)
    }
}
