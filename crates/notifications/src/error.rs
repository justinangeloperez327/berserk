use std::{error::Error, fmt};

pub type Result<T> = std::result::Result<T, NotificationError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channel {
    Mail,
    Webhook,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Configuration,
    InvalidMessage,
    MissingTransport,
    Transport,
    Rejected,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationError {
    kind: ErrorKind,
    channel: Option<Channel>,
    message: String,
}
impl NotificationError {
    pub fn new(kind: ErrorKind, channel: Option<Channel>, message: impl Into<String>) -> Self {
        Self {
            kind,
            channel,
            message: message.into(),
        }
    }
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }
    pub const fn channel(&self) -> Option<Channel> {
        self.channel
    }
    pub fn message(&self) -> &str {
        &self.message
    }
}
impl fmt::Display for NotificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl Error for NotificationError {}
