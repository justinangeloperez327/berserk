use super::ModelQuery;
use crate::{Connection, Result, Row, Value};

/// A typed database record. Implementations stay ordinary Rust and make
/// row-to-field conversion explicit.
pub trait Model: Sized {
    const TABLE: &'static str;
    const PRIMARY_KEY: &'static str = "id";

    fn from_row(row: &Row) -> Result<Self>;
    fn key(&self) -> Value;

    fn query() -> ModelQuery<Self> {
        ModelQuery::new()
    }

    fn all(connection: &mut dyn Connection) -> Result<Vec<Self>> {
        Self::query().get(connection)
    }

    fn find(connection: &mut dyn Connection, key: impl Into<Value>) -> Result<Option<Self>> {
        Self::query()
            .where_(Self::PRIMARY_KEY, "=", key)
            .first(connection)
    }
}
