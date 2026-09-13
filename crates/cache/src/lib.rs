//! Bounded cache contracts and an in-memory implementation.
#![forbid(unsafe_code)]

mod error;
mod memory;
mod namespace;

pub use error::{CacheError, Result};
pub use memory::MemoryCache;
pub use namespace::Namespaced;

use std::{sync::Arc, time::Duration};

pub trait Cache: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    fn put(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()>;
    fn add(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<bool>;
    fn forget(&self, key: &str) -> Result<bool>;
    fn increment(&self, key: &str, amount: i64, ttl: Option<Duration>) -> Result<i64>;

    fn remember<F>(&self, key: &str, ttl: Option<Duration>, load: F) -> Result<Vec<u8>>
    where
        Self: Sized,
        F: FnOnce() -> Result<Vec<u8>>,
    {
        if let Some(value) = self.get(key)? {
            return Ok(value);
        }
        let value = load()?;
        self.put(key, value.clone(), ttl)?;
        Ok(value)
    }
}

impl<C: Cache + ?Sized> Cache for Arc<C> {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        (**self).get(key)
    }
    fn put(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        (**self).put(key, value, ttl)
    }
    fn add(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<bool> {
        (**self).add(key, value, ttl)
    }
    fn forget(&self, key: &str) -> Result<bool> {
        (**self).forget(key)
    }
    fn increment(&self, key: &str, amount: i64, ttl: Option<Duration>) -> Result<i64> {
        (**self).increment(key, amount, ttl)
    }
}

pub(crate) fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() || key.len() > 256 || key.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(CacheError::new(
            "cache keys must contain 1 to 256 non-control bytes",
        ));
    }
    Ok(())
}
