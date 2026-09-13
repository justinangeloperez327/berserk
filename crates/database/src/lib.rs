//! Driver-neutral database contracts.
#![forbid(unsafe_code)]

mod capability;
mod connection;
mod error;
pub mod migrations;
pub mod model;
mod pagination;
pub mod query;
mod row;
mod seeding;
mod statement;
mod transaction;
mod value;

pub mod drivers;

pub use capability::{Capabilities, Capability, Driver};
pub use connection::Connection;
pub use error::{DatabaseError, ErrorKind, Result};
pub use migrations::{AppliedMigration, Migration, MigrationReport, MigrationRunner};
pub use model::{field, BelongsTo, FromValue, HasMany, HasOne, Model, ModelQuery, RelatedSet};
pub use pagination::Page;
pub use query::{Builder as Query, Direction, JoinKind, RawQuery};
pub use row::{Column, Row};
pub use seeding::{run_seeders, Factory, Seeder};
pub use statement::{Execution, Statement};
pub use transaction::{Transaction, TransactionOptions};
pub use value::{Value, ValueRef};

/// Reports whether a driver boundary is enabled in this build.
/// Concrete implementations arrive in Phases 14 and 15.
pub const fn driver_enabled(driver: Driver) -> bool {
    match driver {
        Driver::Postgres => cfg!(feature = "postgres"),
        Driver::MySql => cfg!(feature = "mysql"),
        Driver::Sqlite => cfg!(feature = "sqlite"),
    }
}
