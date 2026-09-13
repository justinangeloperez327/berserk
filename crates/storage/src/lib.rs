//! Safe storage contracts with memory and local-filesystem implementations.
#![forbid(unsafe_code)]

mod error;
mod local;
mod memory;
mod path;

pub use error::{ErrorKind, Result, StorageError};
pub use local::LocalStorage;
pub use memory::MemoryStorage;
pub use path::StoragePath;

use std::io::Read;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectMetadata {
    pub path: StoragePath,
    pub bytes: u64,
    pub modified_at: Option<u64>,
}

pub trait Storage: Send + Sync {
    fn put(&self, path: &StoragePath, source: &mut dyn Read) -> Result<ObjectMetadata>;
    fn read(&self, path: &StoragePath) -> Result<Box<dyn Read + Send>>;
    fn metadata(&self, path: &StoragePath) -> Result<Option<ObjectMetadata>>;
    fn delete(&self, path: &StoragePath) -> Result<bool>;
    fn list(&self, prefix: Option<&StoragePath>) -> Result<Vec<ObjectMetadata>>;
    fn exists(&self, path: &StoragePath) -> Result<bool> {
        Ok(self.metadata(path)?.is_some())
    }
}
