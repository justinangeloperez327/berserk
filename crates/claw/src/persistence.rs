use crate::Model;
use berserk_database::{Connection, DatabaseError, ErrorKind, Execution, Result, Value};

/// Opt-in persistence contract for models that can serialize their fields back to the database.
///
/// `Model` already supports typed create/update/delete. Implement this specialized
/// contract when saving an existing instance needs an explicit write mapping,
/// independent of its presentation attributes. The primary key must not be included in
/// `values_for_save`; `save` always uses it as the update filter.
pub trait PersistableModel: Model {
    /// Return the non-primary-key columns that should be written by `save`.
    fn save(&self) -> Result<Execution> {
        berserk_database::scope::with_connection(|c| self.save_on(c))
    }

    fn values_for_save(&self) -> Vec<(&'static str, Value)>;

    /// Persist this model's declared values using its primary key as the update filter.
    fn save_on(&self, connection: &mut dyn Connection) -> Result<Execution> {
        let values = self.values_for_save();
        if values
            .iter()
            .any(|(column, _)| *column == Self::PRIMARY_KEY)
        {
            return Err(DatabaseError::new(
                ErrorKind::Query,
                "save values must not include the model primary key",
            ));
        }

        Self::where_op(Self::PRIMARY_KEY, "=", self.key()).update_on(connection, values)
    }
}
