use std::{error::Error, fmt};

/// A configuration error without potentially sensitive configuration values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    field: &'static str,
    reason: &'static str,
}

impl ConfigError {
    pub fn new(field: &'static str, reason: &'static str) -> Self {
        Self { field, reason }
    }

    pub fn field(&self) -> &'static str {
        self.field
    }
    pub fn reason(&self) -> &'static str {
        self.reason
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid configuration '{}': {}", self.field, self.reason)
    }
}

impl Error for ConfigError {}
