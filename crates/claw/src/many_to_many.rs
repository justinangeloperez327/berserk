use crate::{relationship::unique_non_null, Model, RelatedSet};
use berserk_database::{Connection, DatabaseError, ErrorKind, Query, Result, Value};
use std::marker::PhantomData;

/// Related records connected through a pivot table.
///
/// The relation batches parent keys into one pivot query and related keys into
/// one model query, avoiding an N+1 query per parent.
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

    pub fn load_on(&self, connection: &mut dyn Connection, parents: &[P]) -> Result<RelatedSet<R>> {
        let parent_keys = unique_non_null(parents.iter().map(self.parent_key));
        if parent_keys.is_empty() {
            return Ok(RelatedSet::default());
        }

        let links = Query::table(self.pivot_table)
            .select([self.foreign_pivot_key, self.related_pivot_key])
            .where_in(self.foreign_pivot_key, parent_keys)
            .get(connection)?;

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
            if related != Value::Null && !related_keys.contains(&related) {
                related_keys.push(related.clone());
            }
            pairs.push((parent, related));
        }

        if related_keys.is_empty() {
            return Ok(RelatedSet::default());
        }

        let related_rows = Query::table(R::TABLE)
            .where_in(R::PRIMARY_KEY, related_keys)
            .get(connection)?;
        let mut keyed_rows = Vec::with_capacity(related_rows.len());
        for row in related_rows {
            let model = R::from_row(&row)?;
            keyed_rows.push(((self.related_key)(&model), row));
        }

        let mut result = RelatedSet::default();
        for (parent, related) in pairs {
            let Some((_, row)) = keyed_rows.iter().find(|(key, _)| key == &related) else {
                continue;
            };
            result.insert(parent, R::from_row(row)?);
        }
        Ok(result)
    }
}

