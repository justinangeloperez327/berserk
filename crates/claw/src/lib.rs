//! Claw ORM: Berserk's typed model and relationship layer.
//!
//! Claw depends on the driver-neutral database contracts but owns the ORM
//! abstractions so database connections and model behavior can evolve
//! independently.
#![forbid(unsafe_code)]

mod collection;
mod decode;
mod many_to_many;
mod model;
mod pagination;
mod persistence;
mod query;
mod relationship;
mod route_binding;

pub use collection::Collection;
pub use decode::{field, FromValue};
pub use many_to_many::{BelongsToMany, SyncResult};
pub use model::{Attributes, Model};
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
        field, model_fields, Attributes, BelongsTo, BelongsToMany, Collection, FromValue, HasMany,
        HasOne, Model, ModelQuery, Page, PersistableModel, RelatedSet, ScopedRouteModel,
        SyncResult,
    };
    pub use crate::{DatabaseScope, IntoInsert, IntoUpdate, Relationship, Transaction};
}

mod eager;
mod fields;
mod writes;
pub use berserk_database::scope::{with_scoped_connection, DatabaseScope};
pub use eager::{EagerQuery, Loaded, LoadedPage, Relationship};
pub use writes::{IntoInsert, IntoUpdate};

/// Run a synchronous unit of work using the active connection and transaction.
pub struct Transaction;
impl Transaction {
    pub fn run<T, E: From<DatabaseError>>(
        operation: impl FnOnce() -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E> {
        Self::with_options(Default::default(), operation)
    }
    pub fn with_options<T, E: From<DatabaseError>>(
        options: berserk_database::TransactionOptions,
        operation: impl FnOnce() -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E> {
        berserk_database::scope::transaction(options, operation)
    }
}
