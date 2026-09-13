use crate::ListenerId;
use std::{error::Error, fmt};

pub type Result<T> = std::result::Result<T, DispatchError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Listener,
    ListenerPanic,
    TypeMismatch,
    Unavailable,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchError {
    kind: ErrorKind,
    event: &'static str,
    listener: Option<ListenerId>,
    message: String,
}

impl DispatchError {
    pub fn new(
        kind: ErrorKind,
        event: &'static str,
        listener: Option<ListenerId>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            event,
            listener,
            message: message.into(),
        }
    }
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub const fn event(&self) -> &'static str {
        self.event
    }
    pub const fn listener(&self) -> Option<ListenerId> {
        self.listener
    }
    pub(crate) fn with_context(mut self, event: &'static str, listener: ListenerId) -> Self {
        self.event = event;
        self.listener = Some(listener);
        self
    }
}
impl fmt::Display for DispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl Error for DispatchError {}
