use crate::{Model, ModelQuery};
use berserk_database::{Connection, DatabaseError, ErrorKind, Result, Value};
use std::marker::PhantomData;

/// Eager-loaded related records grouped by their linking key.
/// `len` counts populated groups, not models; absent keys return `None`.
/// Signed and unsigned representations of the same nonnegative integer match.
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
            .find(|(candidate, _)| keys_equal(candidate, key))
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

    pub fn map<T>(self, mut mapper: impl FnMut(M) -> T) -> RelatedSet<T> {
        let mut groups = Vec::with_capacity(self.groups.len());
        for (key, models) in self.groups {
            let mut mapped = Vec::with_capacity(models.len());
            for model in models {
                mapped.push(mapper(model));
            }
            groups.push((key, mapped));
        }
        RelatedSet { groups }
    }

    pub(crate) fn insert(&mut self, key: Value, model: M) {
        if let Some((_, models)) = self
            .groups
            .iter_mut()
            .find(|(candidate, _)| keys_equal(candidate, &key))
        {
            models.push(model);
        } else {
            self.groups.push((key, vec![model]));
        }
    }
}

enum RelatedKey<R> {
    Infallible(fn(&R) -> Value),
    Fallible(fn(&R) -> Result<Value>),
}

impl<R> RelatedKey<R> {
    fn get(&self, model: &R) -> Result<Value> {
        match self {
            Self::Infallible(accessor) => Ok(accessor(model)),
            Self::Fallible(accessor) => accessor(model),
        }
    }
}

enum ChildKey<C> {
    Infallible(fn(&C) -> Option<Value>),
    Fallible(fn(&C) -> Result<Option<Value>>),
}

impl<C> ChildKey<C> {
    fn get(&self, model: &C) -> Result<Option<Value>> {
        match self {
            Self::Infallible(accessor) => Ok(accessor(model)),
            Self::Fallible(accessor) => accessor(model),
        }
    }
}

/// Children grouped by their foreign key. Key accessors stay explicit; no reflection.
pub struct HasMany<P, R> {
    foreign_key: &'static str,
    parent_key: fn(&P) -> Value,
    related_key: RelatedKey<R>,
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
            related_key: RelatedKey::Infallible(related_key),
        }
    }

    /// Construct a relationship whose related-key accessor can report mapping errors.
    pub const fn try_new(
        foreign_key: &'static str,
        parent_key: fn(&P) -> Value,
        related_key: fn(&R) -> Result<Value>,
    ) -> Self {
        Self {
            foreign_key,
            parent_key,
            related_key: RelatedKey::Fallible(related_key),
        }
    }

    /// Build a normal model query restricted to one parent's foreign key.
    pub fn query_for(&self, parent: &P) -> Result<ModelQuery<R>> {
        let key = (self.parent_key)(parent);
        validate_key(&key)?;
        Ok(R::query().scope(|query| query.constrain_in(self.foreign_key, [key])))
    }

    #[deprecated(
        since = "1.0.0",
        note = "use load_on(connection, parents); use Relationship::load for scoped loading"
    )]
    pub fn load(&self, connection: &mut dyn Connection, parents: &[P]) -> Result<RelatedSet<R>> {
        self.load_on(connection, parents)
    }

    pub fn load_on(&self, connection: &mut dyn Connection, parents: &[P]) -> Result<RelatedSet<R>> {
        let keys = unique_non_null(parents.iter().map(self.parent_key));
        if keys.is_empty() {
            return Ok(RelatedSet::default());
        }
        let related = R::query()
            .where_in(self.foreign_key, keys)
            .get_on(connection)?;
        let mut result = RelatedSet::default();
        for model in related {
            result.insert(self.related_key.get(&model)?, model);
        }
        Ok(result)
    }
}

/// Zero or one child per key. Batch loading returns [`RelatedSet`] for 1.x compatibility
/// and reports [`ErrorKind::Decode`] if a key has more than one child.
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

    /// Construct a one-to-one relationship with a fallible related-key accessor.
    pub const fn try_new(
        foreign_key: &'static str,
        parent_key: fn(&P) -> Value,
        related_key: fn(&R) -> Result<Value>,
    ) -> Self {
        Self {
            inner: HasMany::try_new(foreign_key, parent_key, related_key),
        }
    }

    /// Build a normal query. Unlike `load_on`, this does not enforce cardinality.
    pub fn query_for(&self, parent: &P) -> Result<ModelQuery<R>> {
        self.inner.query_for(parent)
    }

    #[deprecated(
        since = "1.0.0",
        note = "use load_on(connection, parents); use Relationship::load for scoped loading"
    )]
    pub fn load(&self, connection: &mut dyn Connection, parents: &[P]) -> Result<RelatedSet<R>> {
        self.load_on(connection, parents)
    }

    pub fn load_on(&self, connection: &mut dyn Connection, parents: &[P]) -> Result<RelatedSet<R>> {
        let result = self.inner.load_on(connection, parents)?;
        if result.groups.iter().any(|(_, models)| models.len() > 1) {
            return Err(DatabaseError::new(
                ErrorKind::Decode,
                "has_one relationship returned multiple records for one key",
            ));
        }
        Ok(result)
    }
}

/// Owners grouped by their owner key. Missing/NULL child keys are skipped when loading.
pub struct BelongsTo<C, R> {
    owner_key: &'static str,
    child_key: ChildKey<C>,
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
            child_key: ChildKey::Infallible(child_key),
            related_key,
            marker: PhantomData,
        }
    }

    /// Construct a belongs-to relationship whose child-key accessor can report
    /// model metadata errors.
    pub const fn try_new(
        owner_key: &'static str,
        child_key: fn(&C) -> Result<Option<Value>>,
        related_key: fn(&R) -> Value,
    ) -> Self {
        Self {
            owner_key,
            child_key: ChildKey::Fallible(child_key),
            related_key,
            marker: PhantomData,
        }
    }

    /// Build an owner query. An absent nullable foreign key matches no records.
    pub fn query_for(&self, child: &C) -> Result<ModelQuery<R>> {
        let key = self.child_key.get(child)?.filter(|key| *key != Value::Null);
        if let Some(key) = &key {
            validate_key(key)?;
        }
        Ok(R::query().scope(|query| query.constrain_in(self.owner_key, key)))
    }

    #[deprecated(
        since = "1.0.0",
        note = "use load_on(connection, children); use Relationship::load for scoped loading"
    )]
    pub fn load(&self, connection: &mut dyn Connection, children: &[C]) -> Result<RelatedSet<R>> {
        self.load_on(connection, children)
    }

    pub fn load_on(
        &self,
        connection: &mut dyn Connection,
        children: &[C],
    ) -> Result<RelatedSet<R>> {
        let mut child_keys = Vec::with_capacity(children.len());
        for child in children {
            if let Some(key) = self.child_key.get(child)? {
                child_keys.push(key);
            }
        }
        let keys = unique_non_null(child_keys);
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

pub(crate) fn unique_non_null(values: impl IntoIterator<Item = Value>) -> Vec<Value> {
    let mut unique = Vec::new();
    for value in values {
        if value != Value::Null && !unique.iter().any(|key| keys_equal(key, &value)) {
            unique.push(value);
        }
    }
    unique
}

// Drivers may decode a positive integer as signed even when a model uses u64.
pub(crate) fn keys_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::I64(signed), Value::U64(unsigned)) | (Value::U64(unsigned), Value::I64(signed)) => {
            u64::try_from(*signed).ok() == Some(*unsigned)
        }
        _ => left == right,
    }
}

pub(crate) fn validate_key(key: &Value) -> Result<()> {
    match key {
        Value::I64(_) | Value::U64(_) => Ok(()),
        Value::Text(value) if !value.is_empty() => Ok(()),
        Value::Bytes(value) if !value.is_empty() => Ok(()),
        _ => Err(DatabaseError::new(
            ErrorKind::InvalidInput,
            "relationship keys must be integers or nonempty text/bytes",
        )),
    }
}
