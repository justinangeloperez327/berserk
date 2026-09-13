use std::{error::Error, fmt};

pub type Result<T> = std::result::Result<T, ClientError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    InvalidUrl,
    InvalidRequest,
    UnsupportedScheme,
    Dns,
    Connect,
    Timeout,
    Write,
    MalformedResponse,
    ResponseTooLarge,
    Io,
    Transport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientError {
    kind: ErrorKind,
    message: String,
}
impl ClientError {
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
}
impl fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl Error for ClientError {}
