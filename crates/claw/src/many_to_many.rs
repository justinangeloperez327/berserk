use crate::{
    relationship::{key_chunks, keys_equal, unique_non_null, validate_key},
    Model, ModelQuery, RelatedSet,
};
use berserk_database::{Connection, DatabaseError, ErrorKind, Query, Result, Value};
use std::marker::PhantomData;

mod pivot;
pub use pivot::SyncResult;

/// Related records connected through a pivot table.
///
/// The relation batches parent keys into one pivot query and related keys into
/// one model query, avoiding an N+1 query per parent.
/// Duplicate pivot rows are preserved. The related key accessor must return the
/// value of `R::PRIMARY_KEY`; a database UNIQUE constraint controls link uniqueness.
pub struct BelongsToMany<P, R> {
    pivot_table: &'static str,
    foreign_pivot_key: &'static str,
    related_pivot_key: &'static str,
    parent_key: fn(&P) -> Value,
    related_key: fn(&R) -> Value,
    marker: PhantomData<fn() -> R>,
}

impl<P, R: Model> BelongsToMany<P, R> {
    pub const fn new(
        pivot_table: &'static str,
        foreign_pivot_key: &'static str,
        related_pivot_key: &'static str,
        parent_key: fn(&P) -> Value,
        related_key: fn(&R) -> Value,
    ) -> Self {
        Self {
            pivot_table,
            foreign_pivot_key,
            related_pivot_key,
            parent_key,
            related_key,
            marker: PhantomData,
        }
    }

    /// Build a joined model query preserving actual pivot rows, including duplicates.
    /// Qualify ambiguous columns with the related table name (for example `roles.id`).
    /// Joined updates/deletes are rejected by the portable query builder; use pivot APIs.
    pub fn query_for(&self, parent: &P) -> Result<ModelQuery<R>> {
        let key = (self.parent_key)(parent);
        validate_key(&key)?;
        Ok(R::query()
            .select([format!("{}.*", R::TABLE)])
            .scope(|query| {
                query
                    .join(
                        self.pivot_table,
                        format!("{}.{}", self.pivot_table, self.related_pivot_key),
                        format!("{}.{}", R::TABLE, R::PRIMARY_KEY),
                    )
                    .constrain_in(
                        format!("{}.{}", self.pivot_table, self.foreign_pivot_key),
                        [key],
                    )
            }))
    }

    pub fn load(&self, parents: &[P]) -> Result<RelatedSet<R>> {
        berserk_database::scope::with_connection(|connection| self.load_on(connection, parents))
    }

    pub fn load_on(&self, connection: &mut dyn Connection, parents: &[P]) -> Result<RelatedSet<R>> {
        let parent_keys = unique_non_null(parents.iter().map(self.parent_key));
        if parent_keys.is_empty() {
            return Ok(RelatedSet::default());
        }

        let mut links = Vec::new();
        for chunk in key_chunks(&parent_keys) {
            links.extend(
                Query::table(self.pivot_table)
                    .select([self.foreign_pivot_key, self.related_pivot_key])
                    .where_in(self.foreign_pivot_key, chunk.iter().cloned())
                    .get(connection)?,
            );
        }

        let mut pairs = Vec::with_capacity(links.len());
        let mut related_keys = Vec::new();
        for row in links {
            let parent = row.get(self.foreign_pivot_key).cloned().ok_or_else(|| {
                DatabaseError::new(
                    ErrorKind::Decode,
                    format!(
                        "pivot row is missing foreign key `{}`",
                        self.foreign_pivot_key
                    ),
                )
            })?;
            let related = row.get(self.related_pivot_key).cloned().ok_or_else(|| {
                DatabaseError::new(
                    ErrorKind::Decode,
                    format!(
                        "pivot row is missing related key `{}`",
                        self.related_pivot_key
                    ),
                )
            })?;
            if parent == Value::Null || related == Value::Null {
                return Err(DatabaseError::new(
                    ErrorKind::Decode,
                    "pivot keys cannot be NULL",
                ));
            }
            if !related_keys.iter().any(|key| keys_equal(key, &related)) {
                related_keys.push(related.clone());
            }
            pairs.push((parent, related));
        }

        if related_keys.is_empty() {
            return Ok(RelatedSet::default());
        }

        let mut related_rows = Vec::new();
        for chunk in key_chunks(&related_keys) {
            related_rows.extend(
                Query::table(R::TABLE)
                    .where_in(R::PRIMARY_KEY, chunk.iter().cloned())
                    .get(connection)?,
            );
        }
        let mut keyed_rows = Vec::with_capacity(related_rows.len());
        for row in related_rows {
            let model = R::from_row(&row)?;
            keyed_rows.push(((self.related_key)(&model), row));
        }

        let mut result = RelatedSet::default();
        for (parent, related) in pairs {
            let Some(parent) = parent_keys.iter().find(|key| keys_equal(key, &parent)) else {
                continue;
            };
            let Some((_, row)) = keyed_rows.iter().find(|(key, _)| keys_equal(key, &related))
            else {
                continue;
            };
            result.insert(parent.clone(), R::from_row(row)?);
        }
        Ok(result)
    }
}
