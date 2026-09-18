use crate::Model;
use berserk_database::{Connection, DatabaseError, ErrorKind, Result, Value};
use std::marker::PhantomData;

/// Eager-loaded related records grouped by their linking key.
#[derive(Clone, Debug, PartialEq)]
pub struct RelatedSet<M> {
    groups: Vec<(Value, Vec<M>)>,
}

impl<M> Default for RelatedSet<M> {
    fn default() -> Self {
        Self { groups: Vec::new() }
    }
}

impl<M> RelatedSet<M> {
    pub fn get(&self, key: &Value) -> Option<&[M]> {
        self.groups
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, models)| models.as_slice())
    }

    pub fn groups(&self) -> &[(Value, Vec<M>)] {
        &self.groups
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    fn insert(&mut self, key: Value, model: M) {
        if let Some((_, models)) = self
            .groups
            .iter_mut()
            .find(|(candidate, _)| candidate == &key)
        {
            models.push(model);
        } else {
            self.groups.push((key, vec![model]));
        }
    }
}

pub struct HasMany<P, R> {
    foreign_key: &'static str,
    parent_key: fn(&P) -> Value,
    related_key: fn(&R) -> Value,
}

impl<P, R: Model> HasMany<P, R> {
    pub const fn new(
        foreign_key: &'static str,
        parent_key: fn(&P) -> Value,
        related_key: fn(&R) -> Value,
    ) -> Self {
        Self {
            foreign_key,
            parent_key,
            related_key,
        }
    }

    pub fn load(&self, connection: &mut dyn Connection, parents: &[P]) -> Result<RelatedSet<R>> {
        let keys = unique_non_null(parents.iter().map(self.parent_key));
        if keys.is_empty() {
            return Ok(RelatedSet::default());
        }
        let related = R::query()
            .where_in(self.foreign_key, keys)
            .get_on(connection)?;
        let mut result = RelatedSet::default();
        for model in related {
            result.insert((self.related_key)(&model), model);
        }
        Ok(result)
    }
}

pub struct HasOne<P, R> {
    inner: HasMany<P, R>,
}

impl<P, R: Model> HasOne<P, R> {
    pub const fn new(
        foreign_key: &'static str,
        parent_key: fn(&P) -> Value,
        related_key: fn(&R) -> Value,
    ) -> Self {
        Self {
            inner: HasMany::new(foreign_key, parent_key, related_key),
        }
    }

    pub fn load(&self, connection: &mut dyn Connection, parents: &[P]) -> Result<RelatedSet<R>> {
        let result = self.inner.load(connection, parents)?;
        if result.groups.iter().any(|(_, models)| models.len() > 1) {
            return Err(DatabaseError::new(
                ErrorKind::Decode,
                "has_one relationship returned multiple records for one key",
            ));
        }
        Ok(result)
    }
}

pub struct BelongsTo<C, R> {
    owner_key: &'static str,
    child_key: fn(&C) -> Option<Value>,
    related_key: fn(&R) -> Value,
    marker: PhantomData<fn() -> R>,
}

impl<C, R: Model> BelongsTo<C, R> {
    pub const fn new(
        owner_key: &'static str,
        child_key: fn(&C) -> Option<Value>,
        related_key: fn(&R) -> Value,
    ) -> Self {
        Self {
            owner_key,
            child_key,
            related_key,
            marker: PhantomData,
        }
    }

    pub fn load(&self, connection: &mut dyn Connection, children: &[C]) -> Result<RelatedSet<R>> {
        let keys = unique_non_null(children.iter().filter_map(self.child_key));
        if keys.is_empty() {
            return Ok(RelatedSet::default());
        }
        let related = R::query()
            .where_in(self.owner_key, keys)
            .get_on(connection)?;
        let mut result = RelatedSet::default();
        for model in related {
            result.insert((self.related_key)(&model), model);
        }
        Ok(result)
    }
}

fn unique_non_null(values: impl IntoIterator<Item = Value>) -> Vec<Value> {
    let mut unique = Vec::new();
    for value in values {
        if value != Value::Null && !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}
