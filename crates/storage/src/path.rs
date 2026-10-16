use crate::{ErrorKind, Result, StorageError};
use std::fmt;

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StoragePath(String);

impl StoragePath {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 1024
            || value.starts_with('/')
            || value.contains(['\\', ':', '<', '>', '"', '|', '?', '*'])
            || value
                .bytes()
                .any(|byte| byte == 0 || byte.is_ascii_control())
        {
            return Err(invalid());
        }
        if value.split('/').any(|part| {
            let upper = part
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            let stem = upper.as_str();
            part.is_empty()
                || part.ends_with(['.', ' '])
                || matches!(stem, "CON" | "PRN" | "AUX" | "NUL")
                || stem.strip_prefix("COM").is_some_and(reserved_device_suffix)
                || stem.strip_prefix("LPT").is_some_and(reserved_device_suffix)
        }) {
            return Err(invalid());
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for StoragePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("StoragePath").field(&self.0).finish()
    }
}
impl fmt::Display for StoragePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn invalid() -> StorageError {
    StorageError::new(
        ErrorKind::InvalidPath,
        "storage paths must be relative normalized paths without traversal, control bytes, or Windows-special components",
    )
}

fn reserved_device_suffix(suffix: &str) -> bool {
    matches!(
        suffix,
        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
    )
}
