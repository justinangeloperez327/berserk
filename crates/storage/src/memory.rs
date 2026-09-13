use crate::{ErrorKind, ObjectMetadata, Result, Storage, StorageError, StoragePath};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
struct Object {
    bytes: Vec<u8>,
    modified_at: u64,
}

pub struct MemoryStorage {
    max_objects: usize,
    max_object_bytes: usize,
    objects: Mutex<BTreeMap<StoragePath, Object>>,
}

impl MemoryStorage {
    pub fn new(max_objects: usize, max_object_bytes: usize) -> Result<Self> {
        if max_objects == 0 || max_object_bytes == 0 {
            return Err(StorageError::new(
                ErrorKind::Capacity,
                "storage limits must be greater than zero",
            ));
        }
        Ok(Self {
            max_objects,
            max_object_bytes,
            objects: Mutex::new(BTreeMap::new()),
        })
    }
}

impl Storage for MemoryStorage {
    fn put(&self, path: &StoragePath, source: &mut dyn Read) -> Result<ObjectMetadata> {
        let bytes = read_bounded(source, self.max_object_bytes)?;
        let mut objects = self
            .objects
            .lock()
            .map_err(|_| StorageError::new(ErrorKind::Io, "memory storage lock poisoned"))?;
        if !objects.contains_key(path) && objects.len() >= self.max_objects {
            return Err(StorageError::new(
                ErrorKind::Capacity,
                "memory storage object capacity reached",
            ));
        }
        let modified_at = now()?;
        objects.insert(
            path.clone(),
            Object {
                bytes: bytes.clone(),
                modified_at,
            },
        );
        Ok(ObjectMetadata {
            path: path.clone(),
            bytes: bytes.len() as u64,
            modified_at: Some(modified_at),
        })
    }
    fn read(&self, path: &StoragePath) -> Result<Box<dyn Read + Send>> {
        let objects = self
            .objects
            .lock()
            .map_err(|_| StorageError::new(ErrorKind::Io, "memory storage lock poisoned"))?;
        let bytes = objects
            .get(path)
            .ok_or_else(|| StorageError::new(ErrorKind::NotFound, "stored object not found"))?
            .bytes
            .clone();
        Ok(Box::new(Cursor::new(bytes)))
    }
    fn metadata(&self, path: &StoragePath) -> Result<Option<ObjectMetadata>> {
        let objects = self
            .objects
            .lock()
            .map_err(|_| StorageError::new(ErrorKind::Io, "memory storage lock poisoned"))?;
        Ok(objects.get(path).map(|object| ObjectMetadata {
            path: path.clone(),
            bytes: object.bytes.len() as u64,
            modified_at: Some(object.modified_at),
        }))
    }
    fn delete(&self, path: &StoragePath) -> Result<bool> {
        Ok(self
            .objects
            .lock()
            .map_err(|_| StorageError::new(ErrorKind::Io, "memory storage lock poisoned"))?
            .remove(path)
            .is_some())
    }
    fn list(&self, prefix: Option<&StoragePath>) -> Result<Vec<ObjectMetadata>> {
        let objects = self
            .objects
            .lock()
            .map_err(|_| StorageError::new(ErrorKind::Io, "memory storage lock poisoned"))?;
        Ok(objects
            .iter()
            .filter(|(path, _)| {
                prefix.is_none_or(|prefix| {
                    path.as_str() == prefix.as_str()
                        || path
                            .as_str()
                            .strip_prefix(prefix.as_str())
                            .is_some_and(|rest| rest.starts_with('/'))
                })
            })
            .map(|(path, object)| ObjectMetadata {
                path: path.clone(),
                bytes: object.bytes.len() as u64,
                modified_at: Some(object.modified_at),
            })
            .collect())
    }
}

pub(crate) fn read_bounded(source: &mut dyn Read, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = source.read(&mut buffer).map_err(StorageError::io)?;
        if read == 0 {
            break;
        }
        if bytes
            .len()
            .checked_add(read)
            .is_none_or(|size| size > limit)
        {
            return Err(StorageError::new(
                ErrorKind::TooLarge,
                "stored object exceeds configured byte limit",
            ));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn now() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .map_err(|_| StorageError::new(ErrorKind::Io, "system clock is before Unix epoch"))
}
