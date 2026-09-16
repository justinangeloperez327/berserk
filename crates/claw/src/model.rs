use crate::ModelQuery;
use framework_database::{Connection, Direction, Execution, Query, Result, Row, Value};

/// A typed database record managed by Claw ORM.
pub trait Model: Sized {
    const TABLE: &'static str;
    const PRIMARY_KEY: &'static str = "id";

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

    fn where_(
        column: impl Into<String>,
        operator: impl Into<String>,
        value: impl Into<Value>,
    ) -> ModelQuery<Self> {
        Self::query().where_(column, operator, value)
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

    fn where_null(column: impl Into<String>) -> ModelQuery<Self> {
        Self::query().where_null(column)
    }

    fn where_not_null(column: impl Into<String>) -> ModelQuery<Self> {
        Self::query().where_not_null(column)
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

    fn all(connection: &mut dyn Connection) -> Result<Vec<Self>> {
        Self::query().get(connection)
    }

    fn count(connection: &mut dyn Connection) -> Result<u64> {
        Self::query().count(connection)
    }

    fn exists(connection: &mut dyn Connection) -> Result<bool> {
        Self::query().exists(connection)
    }

    fn find(connection: &mut dyn Connection, key: impl Into<Value>) -> Result<Option<Self>> {
        Self::where_(Self::PRIMARY_KEY, "=", key).first(connection)
    }

    fn find_many<I, V>(connection: &mut dyn Connection, keys: I) -> Result<Vec<Self>>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        Self::where_in(Self::PRIMARY_KEY, keys).get(connection)
    }

    fn create<I, S, V>(connection: &mut dyn Connection, values: I) -> Result<Execution>
    where
        I: IntoIterator<Item = (S, V)>,
        S: Into<String>,
        V: Into<Value>,
    {
        Query::table(Self::TABLE).insert(values).execute(connection)
    }

    fn destroy(connection: &mut dyn Connection, key: impl Into<Value>) -> Result<Execution> {
        Self::where_(Self::PRIMARY_KEY, "=", key).delete(connection)
    }

    /// Persist explicit values for this model's primary-key row.
    ///
    /// This does not mutate the in-memory model because Claw does not yet
    /// assume a generic mapping from database column names back to struct
    /// fields.
    fn update<I, S, V>(&self, connection: &mut dyn Connection, values: I) -> Result<Execution>
    where
        I: IntoIterator<Item = (S, V)>,
        S: Into<String>,
        V: Into<Value>,
    {
        Self::where_(Self::PRIMARY_KEY, "=", self.key()).update(connection, values)
    }

    /// Delete this model's primary-key row.
    fn delete(&self, connection: &mut dyn Connection) -> Result<Execution> {
        Self::where_(Self::PRIMARY_KEY, "=", self.key()).delete(connection)
    }
}
