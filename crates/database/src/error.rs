use crate::{Capability, Driver};
use std::{error::Error, fmt};

pub type Result<T> = std::result::Result<T, DatabaseError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Configuration,
    NotFound,
    InvalidInput,
    Connection,
    Timeout,
    Constraint,
    /// A UNIQUE or primary-key constraint rejected the write.
    UniqueViolation,
    /// A FOREIGN KEY constraint rejected the write.
    ForeignKeyViolation,
    /// A NOT NULL constraint rejected the write.
    NotNullViolation,
    Serialization,
    Query,
    Decode,
    Transaction,
    Unsupported {
        driver: Driver,
        capability: Capability,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseError {
    kind: ErrorKind,
    message: String,
    code: Option<String>,
}

impl DatabaseError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            code: None,
        }
    }
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Returns true for any database integrity-constraint failure.
    pub fn is_constraint_violation(&self) -> bool {
        matches!(
            self.kind,
            ErrorKind::Constraint
                | ErrorKind::UniqueViolation
                | ErrorKind::ForeignKeyViolation
                | ErrorKind::NotNullViolation
        )
    }

    /// Returns true when retrying the same operation may succeed after transient contention.
    pub fn is_retryable(&self) -> bool {
        matches!(self.kind, ErrorKind::Timeout | ErrorKind::Serialization)
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl Error for DatabaseError {}
