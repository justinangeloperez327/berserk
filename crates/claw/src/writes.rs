use crate::Model;
use berserk_database::{
    Capability, Connection, DatabaseError, ErrorKind, Query, Result, Statement, Value,
};

/// Explicit conversion from trusted, validated input to insertable model columns.
pub trait IntoInsert<M: Model> {
    fn into_insert(self) -> Result<Vec<(String, Value)>>;
}
/// Explicit conversion from trusted, validated input to updatable model columns.
pub trait IntoUpdate<M: Model> {
    fn into_update(self) -> Result<Vec<(String, Value)>>;
}
impl<M: Model, const N: usize> IntoInsert<M> for [(&str, Value); N] {
    fn into_insert(self) -> Result<Vec<(String, Value)>> {
        Ok(self.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}
impl<M: Model, const N: usize> IntoUpdate<M> for [(&str, Value); N] {
    fn into_update(self) -> Result<Vec<(String, Value)>> {
        Ok(self.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}
pub(crate) fn allowed_values<M: Model>(
    values: Vec<(String, Value)>,
) -> Result<Vec<(String, Value)>> {
    let mut seen = std::collections::HashSet::new();
    for (column, _) in &values {
        if column == M::PRIMARY_KEY
            || !M::FILLABLE.contains(&column.as_str())
            || !seen.insert(column)
        {
            return Err(DatabaseError::new(
                ErrorKind::InvalidInput,
                "column is guarded or repeated",
            ));
        }
    }
    Ok(values)
}
pub(crate) fn not_found() -> DatabaseError {
    DatabaseError::new(ErrorKind::NotFound, "model not found")
}
pub(crate) fn insert_model<M: Model>(
    c: &mut dyn Connection,
    values: Vec<(String, Value)>,
) -> Result<M> {
    let statement = Query::table(M::TABLE)
        .insert(values)
        .to_statement(c.driver())?;
    if c.capabilities().supports(Capability::Returning) {
        let mut returning = Statement::new(format!("{} RETURNING *", statement.sql()));
        for value in statement.bindings() {
            returning = returning.bind(value.clone());
        }
        let rows = c.query(&returning)?;
        rows.first().ok_or_else(not_found).and_then(M::from_row)
    } else {
        let result = c.execute(&statement)?;
        let key = result
            .last_insert_id
            .ok_or_else(|| DatabaseError::new(ErrorKind::Query, "driver returned no insert key"))?;
        M::find_on(c, key)?.ok_or_else(not_found)
    }
}
