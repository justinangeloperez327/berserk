use crate::{
    Attributes, BelongsTo, BelongsToMany, Collection, Connection, HasMany, HasOne, Model,
    ModelQuery, Page, RelatedSet, Result, Value,
};

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
}

impl NamedRelation {
    pub fn one(name: impl Into<String>, groups: RelatedSet<Attributes>) -> Self {
        Self {
            name: name.into(),
            cardinality: RelationCardinality::One,
            groups,
        }
    }

    pub fn many(name: impl Into<String>, groups: RelatedSet<Attributes>) -> Self {
        Self {
            name: name.into(),
            cardinality: RelationCardinality::Many,
            groups,
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

/// A parent page and relationships loaded only for that page's items.
pub struct LoadedPage<M, R> {
    pub page: Page<M>,
    pub relations: R,
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
    let mut relations = NamedRelations::new();
    for name in names {
        relations.push(M::load_named_relation(name, connection, models)?);
    }
    Ok(relations)
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

impl<'a, M: Model, const N: usize> IntoEager<M, NamedEager> for [&'a str; N] {
    type Query = NamedEagerQuery<M>;

    fn into_eager(self, query: ModelQuery<M>) -> Self::Query {
        NamedEagerQuery::new(query, self)
    }
}

impl<'a, M: Model> IntoEager<M, NamedEager> for &'a str {
    type Query = NamedEagerQuery<M>;

    fn into_eager(self, query: ModelQuery<M>) -> Self::Query {
        NamedEagerQuery::new(query, [self])
    }
}

impl<'a, M: Model> IntoEager<M, NamedEager> for Vec<&'a str> {
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
