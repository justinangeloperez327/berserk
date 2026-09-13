use crate::{validate_key, Cache, CacheError, Result};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
struct Entry {
    value: Vec<u8>,
    expires_at: Option<u64>,
    accessed: u64,
}

#[derive(Debug, Default)]
struct State {
    entries: HashMap<String, Entry>,
    clock: u64,
}

pub struct MemoryCache {
    capacity: usize,
    max_value_bytes: usize,
    state: Mutex<State>,
}

impl std::fmt::Debug for MemoryCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entry_count = self.state.lock().map(|state| state.entries.len()).ok();
        f.debug_struct("MemoryCache")
            .field("capacity", &self.capacity)
            .field("max_value_bytes", &self.max_value_bytes)
            .field("entry_count", &entry_count)
            .finish()
    }
}

impl MemoryCache {
    pub fn new(capacity: usize, max_value_bytes: usize) -> Result<Self> {
        if capacity == 0 || max_value_bytes == 0 {
            return Err(CacheError::new(
                "cache capacity and value limit must be greater than zero",
            ));
        }
        Ok(Self {
            capacity,
            max_value_bytes,
            state: Mutex::new(State::default()),
        })
    }
    pub fn len(&self) -> Result<usize> {
        let now = Self::now()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| CacheError::new("cache lock poisoned"))?;
        state
            .entries
            .retain(|_, entry| entry.expires_at.is_none_or(|expiry| expiry > now));
        Ok(state.entries.len())
    }
    pub fn is_empty(&self) -> Result<bool> {
        self.len().map(|len| len == 0)
    }
    fn now() -> Result<u64> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CacheError::new("system clock is before Unix epoch"))?
            .as_millis()
            .try_into()
            .map_err(|_| CacheError::new("system clock value overflow"))
    }
    fn expiry(now: u64, ttl: Option<Duration>) -> Result<Option<u64>> {
        ttl.map(|value| {
            let millis: u64 = value
                .as_millis()
                .try_into()
                .map_err(|_| CacheError::new("cache expiry overflow"))?;
            now.checked_add(millis)
                .ok_or_else(|| CacheError::new("cache expiry overflow"))
        })
        .transpose()
    }
    fn write(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<Duration>,
        only_absent: bool,
    ) -> Result<bool> {
        validate_key(key)?;
        if value.len() > self.max_value_bytes {
            return Err(CacheError::new("cache value exceeds configured byte limit"));
        }
        let now = Self::now()?;
        let expires_at = Self::expiry(now, ttl)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| CacheError::new("cache lock poisoned"))?;
        state
            .entries
            .retain(|_, entry| entry.expires_at.is_none_or(|expiry| expiry > now));
        if only_absent && state.entries.contains_key(key) {
            return Ok(false);
        }
        if !state.entries.contains_key(key) && state.entries.len() >= self.capacity {
            if let Some(oldest) = state
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.accessed)
                .map(|(key, _)| key.clone())
            {
                state.entries.remove(&oldest);
            }
        }
        state.clock = state.clock.saturating_add(1);
        let accessed = state.clock;
        state.entries.insert(
            key.into(),
            Entry {
                value,
                expires_at,
                accessed,
            },
        );
        Ok(true)
    }
}

impl Cache for MemoryCache {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        validate_key(key)?;
        let now = Self::now()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| CacheError::new("cache lock poisoned"))?;
        if state
            .entries
            .get(key)
            .is_some_and(|entry| entry.expires_at.is_some_and(|expiry| expiry <= now))
        {
            state.entries.remove(key);
            return Ok(None);
        }
        state.clock = state.clock.saturating_add(1);
        let accessed = state.clock;
        Ok(state.entries.get_mut(key).map(|entry| {
            entry.accessed = accessed;
            entry.value.clone()
        }))
    }
    fn put(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        self.write(key, value, ttl, false).map(|_| ())
    }
    fn add(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<bool> {
        self.write(key, value, ttl, true)
    }
    fn forget(&self, key: &str) -> Result<bool> {
        validate_key(key)?;
        Ok(self
            .state
            .lock()
            .map_err(|_| CacheError::new("cache lock poisoned"))?
            .entries
            .remove(key)
            .is_some())
    }
    fn increment(&self, key: &str, amount: i64, ttl: Option<Duration>) -> Result<i64> {
        validate_key(key)?;
        let now = Self::now()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| CacheError::new("cache lock poisoned"))?;
        state
            .entries
            .retain(|_, entry| entry.expires_at.is_none_or(|expiry| expiry > now));
        let current = state
            .entries
            .get(key)
            .map(|entry| {
                std::str::from_utf8(&entry.value)
                    .ok()
                    .and_then(|value| value.parse::<i64>().ok())
                    .ok_or_else(|| CacheError::new("cached counter is not a signed integer"))
            })
            .transpose()?
            .unwrap_or(0);
        let next = current
            .checked_add(amount)
            .ok_or_else(|| CacheError::new("cached counter overflow"))?;
        let expires_at = state
            .entries
            .get(key)
            .and_then(|entry| entry.expires_at)
            .or(Self::expiry(now, ttl)?);
        if !state.entries.contains_key(key) && state.entries.len() >= self.capacity {
            if let Some(oldest) = state
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.accessed)
                .map(|(key, _)| key.clone())
            {
                state.entries.remove(&oldest);
            }
        }
        state.clock = state.clock.saturating_add(1);
        let accessed = state.clock;
        state.entries.insert(
            key.into(),
            Entry {
                value: next.to_string().into_bytes(),
                expires_at,
                accessed,
            },
        );
        Ok(next)
    }
}
