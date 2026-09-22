use crate::{
    Attributes, BelongsTo, BelongsToMany, Collection, Connection, Direction, Driver, HasMany,
    HasOne, Model, ModelQuery, Page, RelatedSet, Result, Statement, Value,
};
use berserk_database::Query;

/// Maximum number of relationship segments accepted by one named eager-load path.
pub const MAX_NAMED_EAGER_DEPTH: usize = 4;
/// Maximum number of unique named eager-load paths accepted by one query level.
pub const MAX_NAMED_EAGER_PATHS: usize = 32;

/// A batch loader used by typed eager loading.
pub trait Relationship<M: Model> {
    type Output;

    fn load_on(&self, connection: &mut dyn Connection, models: &[M]) -> Result<Self::Output>;

    fn load(&self, models: &[M]) -> Result<Self::Output> {
        berserk_database::scope::with_connection(|connection| self.load_on(connection, models))
    }
}

macro_rules! relationship {
    ($name:ident) => {
        impl<M: Model, R: Model> Relationship<M> for $name<M, R> {
            type Output = RelatedSet<R>;

            fn load_on(
                &self,
                connection: &mut dyn Connection,
                models: &[M],
            ) -> Result<Self::Output> {
                self.load_on(connection, models)
            }
        }
    };
}

relationship!(HasMany);
relationship!(HasOne);
relationship!(BelongsTo);
relationship!(BelongsToMany);

impl<M: Model, A: Relationship<M>, B: Relationship<M>> Relationship<M> for (A, B) {
    type Output = (A::Output, B::Output);

    fn load_on(&self, connection: &mut dyn Connection, models: &[M]) -> Result<Self::Output> {
        Ok((
            self.0.load_on(connection, models)?,
            self.1.load_on(connection, models)?,
        ))
    }
}

/// Cardinality used by named eager-loaded relationships.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationCardinality {
    One,
    Many,
}

/// One named relationship loaded for a parent model set.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedRelation {
    name: String,
    cardinality: RelationCardinality,
    groups: RelatedSet<Attributes>,
    keys: RelatedSet<Value>,
    nested: NamedRelations,
}

impl NamedRelation {
    pub fn one(name: impl Into<String>, groups: RelatedSet<Attributes>) -> Self {
        Self {
            name: name.into(),
            cardinality: RelationCardinality::One,
            groups,
            keys: RelatedSet::default(),
            nested: NamedRelations::new(),
        }
    }

    pub fn many(name: impl Into<String>, groups: RelatedSet<Attributes>) -> Self {
        Self {
            name: name.into(),
            cardinality: RelationCardinality::Many,
            groups,
            keys: RelatedSet::default(),
            nested: NamedRelations::new(),
        }
    }

    fn loaded(
        name: impl Into<String>,
        cardinality: RelationCardinality,
        groups: RelatedSet<Attributes>,
        keys: RelatedSet<Value>,
        nested: NamedRelations,
    ) -> Self {
        Self {
            name: name.into(),
            cardinality,
            groups,
            keys,
            nested,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn cardinality(&self) -> RelationCardinality {
        self.cardinality
    }

    pub fn get(&self, parent_key: &Value) -> Option<&[Attributes]> {
        self.groups.get(parent_key)
    }

    /// Related model keys aligned with the values returned by `get`.
    ///
    /// Flat relations created through `one`/`many` do not retain keys because
    /// they have no nested relationship data to address.
    pub fn keys(&self, parent_key: &Value) -> Option<&[Value]> {
        self.keys.get(parent_key)
    }

    pub fn nested(&self) -> &NamedRelations {
        &self.nested
    }
}

/// Named eager-loaded relationship data for a parent collection.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NamedRelations {
    relations: Vec<NamedRelation>,
}

impl NamedRelations {
    pub const fn new() -> Self {
        Self {
            relations: Vec::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&NamedRelation> {
        self.relations
            .iter()
            .find(|relation| relation.name() == name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &NamedRelation> {
        self.relations.iter()
    }

    pub fn len(&self) -> usize {
        self.relations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.relations.is_empty()
    }

    pub(crate) fn push(&mut self, relation: NamedRelation) {
        self.relations.push(relation);
    }
}

/// Parent models and their separately owned, eagerly loaded relationship output.
pub struct Loaded<M, R> {
    pub models: Collection<M>,
    pub relations: R,
}

impl<M, R> Loaded<M, R> {
    pub fn models(&self) -> &Collection<M> {
        &self.models
    }

    pub fn relations(&self) -> &R {
        &self.relations
    }

    pub fn into_parts(self) -> (Collection<M>, R) {
        (self.models, self.relations)
    }
}

impl<M, R> std::ops::Deref for Loaded<M, R> {
    type Target = Collection<M>;

    fn deref(&self) -> &Self::Target {
        &self.models
    }
}

impl<'a, M, R> IntoIterator for &'a Loaded<M, R> {
    type Item = &'a M;
    type IntoIter = std::slice::Iter<'a, M>;

    fn into_iter(self) -> Self::IntoIter {
        self.models.iter()
    }
}

/// A parent page and relationships loaded only for that page's items.
pub struct LoadedPage<M, R> {
    pub page: Page<M>,
    pub relations: R,
}

impl<M, R> LoadedPage<M, R> {
    pub fn page(&self) -> &Page<M> {
        &self.page
    }

    pub fn relations(&self) -> &R {
        &self.relations
    }

    pub fn items(&self) -> &[M] {
        self.page.items()
    }

    pub fn collection(&self) -> &Collection<M> {
        self.page.collection()
    }

    pub fn into_parts(self) -> (Page<M>, R) {
        (self.page, self.relations)
    }

    pub const fn current_page(&self) -> u64 {
        self.page.page()
    }

    pub const fn per_page(&self) -> u64 {
        self.page.per_page()
    }

    pub const fn total(&self) -> u64 {
        self.page.total()
    }

    pub const fn last_page(&self) -> u64 {
        self.page.last_page()
    }

    pub const fn has_previous(&self) -> bool {
        self.page.has_previous()
    }

    pub const fn has_next(&self) -> bool {
        self.page.has_next()
    }
}

impl<M, R> std::ops::Deref for LoadedPage<M, R> {
    type Target = Page<M>;

    fn deref(&self) -> &Self::Target {
        &self.page
    }
}

impl<'a, M, R> IntoIterator for &'a LoadedPage<M, R> {
    type Item = &'a M;
    type IntoIter = std::slice::Iter<'a, M>;

    fn into_iter(self) -> Self::IntoIter {
        self.page.items().iter()
    }
}

/// Explicit typed eager-loading execution for a normal ModelQuery.
pub struct EagerQuery<M, R> {
    pub(crate) query: ModelQuery<M>,
    pub(crate) relations: R,
}

impl<M: Model, R: Relationship<M>> EagerQuery<M, R> {
    pub fn with<S: Relationship<M>>(self, relation: S) -> EagerQuery<M, (R, S)> {
        EagerQuery {
            query: self.query,
            relations: (self.relations, relation),
        }
    }

    pub fn get(self) -> Result<Loaded<M, R::Output>> {
        berserk_database::scope::with_connection(|connection| self.get_on(connection))
    }

    pub fn get_on(self, connection: &mut dyn Connection) -> Result<Loaded<M, R::Output>> {
        let models = self.query.get_on(connection)?;
        let relations = self.relations.load_on(connection, &models)?;
        Ok(Loaded { models, relations })
    }

    pub fn paginate(self, per_page: u64) -> Result<LoadedPage<M, R::Output>> {
        let page = berserk_database::scope::current_page()?;
        berserk_database::scope::with_connection(|connection| {
            self.paginate_on(connection, page, per_page)
        })
    }

    pub fn paginate_on(
        self,
        connection: &mut dyn Connection,
        page: u64,
        per_page: u64,
    ) -> Result<LoadedPage<M, R::Output>> {
        let page = self.query.paginate_on(connection, page, per_page)?;
        let relations = self.relations.load_on(connection, page.items())?;
        Ok(LoadedPage { page, relations })
    }
}

/// Laravel-style eager loading by declared relationship name.
pub struct NamedEagerQuery<M> {
    pub(crate) query: ModelQuery<M>,
    relations: Vec<String>,
}

impl<M: Model> NamedEagerQuery<M> {
    pub(crate) fn new<I, S>(query: ModelQuery<M>, relations: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut eager = Self {
            query,
            relations: Vec::new(),
        };
        eager.extend(relations);
        eager
    }

    pub fn with<const N: usize>(mut self, relations: [&str; N]) -> Self {
        self.extend(relations);
        self
    }

    pub fn select<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.query = self.query.select(columns);
        self
    }

    pub fn where_(mut self, column: impl Into<String>, value: impl Into<Value>) -> Self {
        self.query = self.query.where_(column, value);
        self
    }

    pub fn or_where(mut self, column: impl Into<String>, value: impl Into<Value>) -> Self {
        self.query = self.query.or_where(column, value);
        self
    }

    pub fn where_op(
        mut self,
        column: impl Into<String>,
        operator: impl Into<String>,
        value: impl Into<Value>,
    ) -> Self {
        self.query = self.query.where_op(column, operator, value);
        self
    }

    pub fn or_where_op(
        mut self,
        column: impl Into<String>,
        operator: impl Into<String>,
        value: impl Into<Value>,
    ) -> Self {
        self.query = self.query.or_where_op(column, operator, value);
        self
    }

    pub fn where_in<I, V>(mut self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.query = self.query.where_in(column, values);
        self
    }

    pub fn where_not_in<I, V>(mut self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.query = self.query.where_not_in(column, values);
        self
    }

    pub fn or_where_in<I, V>(mut self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.query = self.query.or_where_in(column, values);
        self
    }

    pub fn or_where_not_in<I, V>(mut self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.query = self.query.or_where_not_in(column, values);
        self
    }

    pub fn where_between(
        mut self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.query = self.query.where_between(column, lower, upper);
        self
    }

    pub fn or_where_between(
        mut self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.query = self.query.or_where_between(column, lower, upper);
        self
    }

    pub fn where_not_between(
        mut self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.query = self.query.where_not_between(column, lower, upper);
        self
    }

    pub fn or_where_not_between(
        mut self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.query = self.query.or_where_not_between(column, lower, upper);
        self
    }

    pub fn where_null(mut self, column: impl Into<String>) -> Self {
        self.query = self.query.where_null(column);
        self
    }

    pub fn where_not_null(mut self, column: impl Into<String>) -> Self {
        self.query = self.query.where_not_null(column);
        self
    }

    pub fn or_where_null(mut self, column: impl Into<String>) -> Self {
        self.query = self.query.or_where_null(column);
        self
    }

    pub fn or_where_not_null(mut self, column: impl Into<String>) -> Self {
        self.query = self.query.or_where_not_null(column);
        self
    }

    pub fn order_by(mut self, column: impl Into<String>, direction: Direction) -> Self {
        self.query = self.query.order_by(column, direction);
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.query = self.query.limit(limit);
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.query = self.query.offset(offset);
        self
    }

    pub fn scope(mut self, scope: impl FnOnce(Query) -> Query) -> Self {
        self.query = self.query.scope(scope);
        self
    }

    pub fn builder(&self) -> &Query {
        self.query.builder()
    }

    pub fn to_statement(&self, driver: Driver) -> Result<Statement> {
        self.query.to_statement(driver)
    }

    pub fn get(self) -> Result<Loaded<M, NamedRelations>> {
        berserk_database::scope::with_connection(|connection| self.get_on(connection))
    }

    pub fn get_on(self, connection: &mut dyn Connection) -> Result<Loaded<M, NamedRelations>> {
        let models = self.query.get_on(connection)?;
        let relations = load_named::<M>(connection, &models, &self.relations)?;
        Ok(Loaded { models, relations })
    }

    pub fn paginate(self, per_page: u64) -> Result<LoadedPage<M, NamedRelations>> {
        let page = berserk_database::scope::current_page()?;
        berserk_database::scope::with_connection(|connection| {
            self.paginate_on(connection, page, per_page)
        })
    }

    pub fn paginate_on(
        self,
        connection: &mut dyn Connection,
        page: u64,
        per_page: u64,
    ) -> Result<LoadedPage<M, NamedRelations>> {
        let page = self.query.paginate_on(connection, page, per_page)?;
        let relations = load_named::<M>(connection, page.items(), &self.relations)?;
        Ok(LoadedPage { page, relations })
    }

    fn extend<I, S>(&mut self, relations: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for relation in relations {
            let relation = relation.into();
            if !self.relations.iter().any(|existing| existing == &relation) {
                self.relations.push(relation);
            }
        }
    }
}

fn load_named<M: Model>(
    connection: &mut dyn Connection,
    models: &[M],
    names: &[String],
) -> Result<NamedRelations> {
    if names.len() > MAX_NAMED_EAGER_PATHS {
        return Err(eager_error(format!(
            "named eager loading accepts at most {MAX_NAMED_EAGER_PATHS} paths"
        )));
    }

    let mut grouped: Vec<(String, Vec<String>)> = Vec::new();
    for path in names {
        let segments: Vec<_> = path.split('.').collect();
        if segments.is_empty() || segments.iter().any(|segment| segment.is_empty()) {
            return Err(eager_error(format!(
                "invalid eager-load path '{path}'; relationship segments cannot be empty"
            )));
        }
        if segments.len() > MAX_NAMED_EAGER_DEPTH {
            return Err(eager_error(format!(
                "eager-load path '{path}' exceeds the maximum depth of {MAX_NAMED_EAGER_DEPTH}"
            )));
        }

        let root = segments[0].to_owned();
        let nested = (segments.len() > 1).then(|| segments[1..].join("."));
        if let Some((_, children)) = grouped.iter_mut().find(|(name, _)| name == &root) {
            if let Some(nested) = nested {
                if !children.iter().any(|existing| existing == &nested) {
                    children.push(nested);
                }
            }
        } else {
            grouped.push((root, nested.into_iter().collect()));
        }
    }

    let mut relations = NamedRelations::new();
    for (name, nested) in grouped {
        relations.push(M::load_named_relation_with(
            &name, &nested, connection, models,
        )?);
    }
    Ok(relations)
}

fn eager_error(message: impl Into<String>) -> berserk_database::DatabaseError {
    berserk_database::DatabaseError::new(berserk_database::ErrorKind::InvalidInput, message)
}

/// Convert a typed related set into presentation-safe named data while
/// retaining related keys for recursively loaded child relationships.
#[doc(hidden)]
pub fn named_related<R: Model>(
    name: &str,
    cardinality: RelationCardinality,
    connection: &mut dyn Connection,
    related: RelatedSet<R>,
    nested: &[String],
) -> Result<NamedRelation> {
    let mut parent_keys = Vec::new();
    let mut models = Vec::new();
    for (parent_key, group) in related.into_groups() {
        for model in group {
            parent_keys.push(parent_key.clone());
            models.push(model);
        }
    }

    let nested_relations = load_named::<R>(connection, &models, nested)?;
    let mut groups = RelatedSet::default();
    let mut keys = RelatedSet::default();
    for (parent_key, model) in parent_keys.into_iter().zip(models) {
        let key = model.key();
        groups.insert(parent_key.clone(), model.visible_attributes());
        keys.insert(parent_key, key);
    }

    Ok(NamedRelation::loaded(
        name,
        cardinality,
        groups,
        keys,
        nested_relations,
    ))
}

#[doc(hidden)]
pub fn named_belongs_to_with<C, R>(
    name: &str,
    relation: &BelongsTo<C, R>,
    foreign_key: &str,
    connection: &mut dyn Connection,
    models: &[C],
    nested: &[String],
) -> Result<NamedRelation>
where
    C: Model,
    R: Model,
{
    let related = relation.load_on(connection, models)?;
    let mut lookup_keys = Vec::new();
    let mut related_models = Vec::new();
    for (lookup_key, group) in related.into_groups() {
        for model in group {
            lookup_keys.push(lookup_key.clone());
            related_models.push(model);
        }
    }

    let nested_relations = load_named::<R>(connection, &related_models, nested)?;
    let prepared: Vec<_> = lookup_keys
        .into_iter()
        .zip(related_models)
        .map(|(lookup_key, model)| (lookup_key, model.key(), model.visible_attributes()))
        .collect();

    let mut groups = RelatedSet::default();
    let mut keys = RelatedSet::default();
    for model in models {
        let foreign = model
            .attributes()
            .get(foreign_key)
            .cloned()
            .ok_or_else(|| {
                berserk_database::DatabaseError::new(
                    berserk_database::ErrorKind::Decode,
                    format!(
                        "belongs_to relationship requires mapped column '{foreign_key}' on model '{}'",
                        C::TABLE
                    ),
                )
            })?;

        if foreign == Value::Null {
            continue;
        }

        if let Some((_, related_key, attributes)) = prepared
            .iter()
            .find(|(lookup_key, _, _)| crate::relationship::keys_equal(lookup_key, &foreign))
        {
            let parent_key = model.key();
            groups.insert(parent_key.clone(), attributes.clone());
            keys.insert(parent_key, related_key.clone());
        }
    }

    Ok(NamedRelation::loaded(
        name,
        RelationCardinality::One,
        groups,
        keys,
        nested_relations,
    ))
}

#[doc(hidden)]
pub fn named_belongs_to<C, R>(
    relation: &BelongsTo<C, R>,
    foreign_key: &str,
    connection: &mut dyn Connection,
    models: &[C],
) -> Result<RelatedSet<Attributes>>
where
    C: Model,
    R: Model,
{
    let related = relation
        .load_on(connection, models)?
        .map(|model| model.visible_attributes());
    let mut grouped = RelatedSet::default();

    for model in models {
        let foreign = model
            .attributes()
            .get(foreign_key)
            .cloned()
            .ok_or_else(|| {
                berserk_database::DatabaseError::new(
                    berserk_database::ErrorKind::Decode,
                    format!(
                        "belongs_to relationship requires mapped column '{foreign_key}' on model '{}'",
                        C::TABLE
                    ),
                )
            })?;

        if foreign == Value::Null {
            continue;
        }

        if let Some(values) = related.get(&foreign) {
            for value in values {
                grouped.insert(model.key(), value.clone());
            }
        }
    }

    Ok(grouped)
}

/// Adapter used by ModelQuery::with to preserve typed eager loading while
/// also accepting relationship names.
#[doc(hidden)]
pub trait IntoEager<M: Model, Kind> {
    type Query;

    fn into_eager(self, query: ModelQuery<M>) -> Self::Query;
}

#[doc(hidden)]
pub struct TypedEager;

#[doc(hidden)]
pub struct NamedEager;

impl<M, R> IntoEager<M, TypedEager> for R
where
    M: Model,
    R: Relationship<M>,
{
    type Query = EagerQuery<M, R>;

    fn into_eager(self, query: ModelQuery<M>) -> Self::Query {
        EagerQuery {
            query,
            relations: self,
        }
    }
}

impl<M: Model, const N: usize> IntoEager<M, NamedEager> for [&str; N] {
    type Query = NamedEagerQuery<M>;

    fn into_eager(self, query: ModelQuery<M>) -> Self::Query {
        NamedEagerQuery::new(query, self)
    }
}

impl<M: Model> IntoEager<M, NamedEager> for &str {
    type Query = NamedEagerQuery<M>;

    fn into_eager(self, query: ModelQuery<M>) -> Self::Query {
        NamedEagerQuery::new(query, [self])
    }
}

impl<M: Model> IntoEager<M, NamedEager> for Vec<&str> {
    type Query = NamedEagerQuery<M>;

    fn into_eager(self, query: ModelQuery<M>) -> Self::Query {
        NamedEagerQuery::new(query, self)
    }
}

impl<M: Model> IntoEager<M, NamedEager> for Vec<String> {
    type Query = NamedEagerQuery<M>;

    fn into_eager(self, query: ModelQuery<M>) -> Self::Query {
        NamedEagerQuery::new(query, self)
    }
}
