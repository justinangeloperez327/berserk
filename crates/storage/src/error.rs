use std::{error::Error, fmt, io};

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    InvalidPath,
    NotFound,
    AlreadyExists,
    TooLarge,
    Io,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageError {
    kind: ErrorKind,
    message: String,
}
impl StorageError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }
    pub(crate) fn io(error: io::Error) -> Self {
        let kind = match error.kind() {
            io::ErrorKind::NotFound => ErrorKind::NotFound,
            io::ErrorKind::AlreadyExists => ErrorKind::AlreadyExists,
            _ => ErrorKind::Io,
        };
        Self::new(kind, error.to_string())
    }
}
impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl Error for StorageError {}
