use crate::{Connection, Result};
use std::sync::Arc;

type AcquireConnection = dyn Fn() -> Result<Box<dyn Connection + Send>> + Send + Sync;

/// Thread-safe handle for acquiring owned database connections.
///
/// The acquisition function may open a new connection or obtain an owned
/// pooled-connection guard. Acquired connections remain `Send`, while
/// transaction-local connection views are allowed to stay on their worker
/// thread. `Database` itself does not serialize access to a single mutable
/// connection.
#[derive(Clone)]
pub struct Database {
    acquire: Arc<AcquireConnection>,
}

impl Database {
    pub fn new<F, C>(acquire: F) -> Self
    where
        F: Fn() -> Result<C> + Send + Sync + 'static,
        C: Connection + Send + 'static,
    {
        Self {
            acquire: Arc::new(move || {
                acquire().map(|connection| Box::new(connection) as Box<dyn Connection + Send>)
            }),
        }
    }

    pub fn acquire(&self) -> Result<Box<dyn Connection + Send>> {
        (self.acquire)()
    }
}
