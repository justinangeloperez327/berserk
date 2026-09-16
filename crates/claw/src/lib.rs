//! Claw ORM: Berserk's typed model and relationship layer.
//!
//! Claw depends on the driver-neutral database contracts but owns the ORM
//! abstractions so database connections and model behavior can evolve
//! independently.
#![forbid(unsafe_code)]

mod decode;
mod model;
mod pagination;
mod query;
mod relationship;

pub use decode::{field, FromValue};
pub use model::Model;
pub use pagination::Page;
pub use query::ModelQuery;
pub use relationship::{BelongsTo, HasMany, HasOne, RelatedSet};

pub use framework_database::{
    Connection, DatabaseError, Direction, Driver, ErrorKind, Result, Row, Statement, Value,
};

pub mod prelude {
    pub use crate::{
        field, BelongsTo, FromValue, HasMany, HasOne, Model, ModelQuery, Page, RelatedSet,
    };
}
