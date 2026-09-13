//! Explicit email and webhook notification delivery.
#![forbid(unsafe_code)]

mod error;
mod mail;
mod notifier;
mod webhook;

pub use error::{Channel, ErrorKind, NotificationError, Result};
pub use mail::{EmailAddress, MailMessage, MailTransport, MemoryMailTransport};
pub use notifier::{
    DeliveryOutcome, DeliveryPolicy, DeliveryReport, Notification, Notifier, NotifierConfig,
    Recipient,
};
pub use webhook::WebhookMessage;
