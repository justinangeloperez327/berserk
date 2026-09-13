use std::{error::Error, fmt};

pub type Result<T> = std::result::Result<T, CliError>;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Usage,
    InvalidName,
    UnsafePath,
    AlreadyExists,
    Io,
    MigrationUnavailable,
    Clock,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliError {
    kind: ErrorKind,
    message: String,
}
impl CliError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn from_io(error: std::io::Error) -> Self {
        Self::new(ErrorKind::Io, error.to_string())
    }
}
impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl Error for CliError {}
