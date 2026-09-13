use std::{error::Error, fmt};

pub type Result<T> = std::result::Result<T, JobError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Configuration,
    QueueFull,
    Closed,
    Handler,
    HandlerPanic,
    FailureStore,
    Clock,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobError {
    kind: ErrorKind,
    message: String,
}
impl JobError {
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
impl fmt::Display for JobError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl Error for JobError {}
