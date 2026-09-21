use crate::{
    BelongsTo, BelongsToMany, Connection, HasMany, HasOne, Model, ModelQuery, Page, RelatedSet,
    Result,
};

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
    fn load_on(&self, c: &mut dyn Connection, models: &[M]) -> Result<Self::Output> {
        Ok((self.0.load_on(c, models)?, self.1.load_on(c, models)?))
    }
}
pub struct Loaded<M, R> {
    pub models: Vec<M>,
    pub relations: R,
}
pub struct LoadedPage<M, R> {
    pub page: Page<M>,
    pub relations: R,
}
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
        berserk_database::scope::with_connection(|c| self.get_on(c))
    }
    pub fn get_on(self, c: &mut dyn Connection) -> Result<Loaded<M, R::Output>> {
        let models = self.query.get_on(c)?;
        let relations = self.relations.load_on(c, &models)?;
        Ok(Loaded { models, relations })
    }
    pub fn paginate(self, per_page: u64) -> Result<LoadedPage<M, R::Output>> {
        let page = berserk_database::scope::current_page()?;
        berserk_database::scope::with_connection(|c| self.paginate_on(c, page, per_page))
    }
    pub fn paginate_on(
        self,
        c: &mut dyn Connection,
        page: u64,
        per_page: u64,
    ) -> Result<LoadedPage<M, R::Output>> {
        let page = self.query.paginate_on(c, page, per_page)?;
        let relations = self.relations.load_on(c, page.items())?;
        Ok(LoadedPage { page, relations })
    }
}
