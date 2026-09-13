use crate::{validate_key, Cache, Result};
use std::time::Duration;

pub struct Namespaced<C> {
    namespace: String,
    inner: C,
}
impl<C> Namespaced<C> {
    pub fn new(namespace: impl Into<String>, inner: C) -> Result<Self> {
        let namespace = namespace.into();
        validate_key(&namespace)?;
        Ok(Self { namespace, inner })
    }
    pub fn into_inner(self) -> C {
        self.inner
    }
    fn key(&self, key: &str) -> Result<String> {
        validate_key(key)?;
        Ok(format!("{}:{key}", self.namespace))
    }
}
impl<C: Cache> Cache for Namespaced<C> {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.inner.get(&self.key(key)?)
    }
    fn put(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        self.inner.put(&self.key(key)?, value, ttl)
    }
    fn add(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<bool> {
        self.inner.add(&self.key(key)?, value, ttl)
    }
    fn forget(&self, key: &str) -> Result<bool> {
        self.inner.forget(&self.key(key)?)
    }
    fn increment(&self, key: &str, amount: i64, ttl: Option<Duration>) -> Result<i64> {
        self.inner.increment(&self.key(key)?, amount, ttl)
    }
}
