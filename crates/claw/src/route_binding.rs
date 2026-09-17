use crate::{Model, ModelQuery};
use berserk_database::Value;

/// A model that can be safely bound beneath a parent model in a nested route.
///
/// The default implementation scopes the child by both its own primary key and
/// the declared foreign key pointing at the parent model's key. Override
/// `scoped_route_query` when the relationship uses a different lookup shape.
pub trait ScopedRouteModel<P: Model>: Model {
    const PARENT_FOREIGN_KEY: &'static str;

    fn scoped_route_query(parent: &P, key: Value) -> ModelQuery<Self> {
        Self::where_op(Self::PRIMARY_KEY, "=", key).where_op(
            Self::PARENT_FOREIGN_KEY,
            "=",
            parent.key(),
        )
    }
}
