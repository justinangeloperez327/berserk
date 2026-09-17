//! Claw ORM: Berserk's typed model and relationship layer.
//!
//! Claw depends on the driver-neutral database contracts but owns the ORM
//! abstractions so database connections and model behavior can evolve
//! independently.
#![forbid(unsafe_code)]

mod decode;
mod model;
mod pagination;
mod persistence;
mod query;
mod relationship;
mod route_binding;

pub use decode::{field, FromValue};
pub use model::Model;
pub use pagination::Page;
pub use persistence::PersistableModel;
pub use query::ModelQuery;
pub use relationship::{BelongsTo, HasMany, HasOne, RelatedSet};
pub use route_binding::ScopedRouteModel;

pub use berserk_database::{
    Connection, DatabaseError, Direction, Driver, ErrorKind, Result, Row, Statement, Value,
};

pub mod prelude {
    pub use crate::{
        field, BelongsTo, FromValue, HasMany, HasOne, Model, ModelQuery, Page, PersistableModel,
        RelatedSet, ScopedRouteModel,
    };
}
