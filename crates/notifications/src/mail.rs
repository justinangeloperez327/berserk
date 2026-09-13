use crate::{Channel, ErrorKind, NotificationError, Result};
use std::{fmt, sync::Mutex};

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct EmailAddress(String);
impl EmailAddress {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.len() > 320
            || value
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte == b' ')
            || value.matches('@').count() != 1
        {
            return Err(NotificationError::new(
                ErrorKind::InvalidMessage,
                Some(Channel::Mail),
                "email address is invalid",
            ));
        }
        let (local, domain) = value.split_once('@').unwrap();
        if local.is_empty() || domain.is_empty() || domain.starts_with('.') || domain.ends_with('.')
        {
            return Err(NotificationError::new(
                ErrorKind::InvalidMessage,
                Some(Channel::Mail),
                "email address is invalid",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for EmailAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EmailAddress([redacted])")
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct MailMessage {
    from: EmailAddress,
    to: EmailAddress,
    subject: String,
    text: String,
    html: Option<String>,
}
impl MailMessage {
    pub fn text(
        from: EmailAddress,
        to: EmailAddress,
        subject: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<Self> {
        let subject = subject.into();
        let text = text.into();
        if subject.is_empty()
            || subject.len() > 998
            || subject
                .bytes()
                .any(|byte| matches!(byte, b'\r' | b'\n' | 0))
        {
            return Err(NotificationError::new(
                ErrorKind::InvalidMessage,
                Some(Channel::Mail),
                "mail subject is invalid",
            ));
        }
        if text.is_empty() {
            return Err(NotificationError::new(
                ErrorKind::InvalidMessage,
                Some(Channel::Mail),
                "mail text body cannot be empty",
            ));
        }
        Ok(Self {
            from,
            to,
            subject,
            text,
            html: None,
        })
    }
    pub fn html(mut self, html: impl Into<String>) -> Result<Self> {
        let html = html.into();
        if html.is_empty() {
            return Err(NotificationError::new(
                ErrorKind::InvalidMessage,
                Some(Channel::Mail),
                "mail HTML body cannot be empty",
            ));
        }
        self.html = Some(html);
        Ok(self)
    }
    pub fn from(&self) -> &EmailAddress {
        &self.from
    }
    pub fn to(&self) -> &EmailAddress {
        &self.to
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn text_body(&self) -> &str {
        &self.text
    }
    pub fn html_body(&self) -> Option<&str> {
        self.html.as_deref()
    }
    pub fn body_bytes(&self) -> usize {
        self.text
            .len()
            .saturating_add(self.html.as_ref().map_or(0, String::len))
    }
}
impl fmt::Debug for MailMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MailMessage")
            .field("from", &"[redacted]")
            .field("to", &"[redacted]")
            .field("subject_bytes", &self.subject.len())
            .field("text_bytes", &self.text.len())
            .field("has_html", &self.html.is_some())
            .finish()
    }
}

pub trait MailTransport: Send + Sync {
    fn send(&self, message: &MailMessage) -> Result<()>;
}

pub struct MemoryMailTransport {
    capacity: usize,
    sent: Mutex<Vec<MailMessage>>,
}
impl MemoryMailTransport {
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 {
            return Err(NotificationError::new(
                ErrorKind::Configuration,
                Some(Channel::Mail),
                "memory mail capacity must be greater than zero",
            ));
        }
        Ok(Self {
            capacity,
            sent: Mutex::new(Vec::new()),
        })
    }
    pub fn sent(&self) -> Result<Vec<MailMessage>> {
        Ok(self
            .sent
            .lock()
            .map_err(|_| {
                NotificationError::new(
                    ErrorKind::Transport,
                    Some(Channel::Mail),
                    "memory mail transport lock poisoned",
                )
            })?
            .clone())
    }
}
impl MailTransport for MemoryMailTransport {
    fn send(&self, message: &MailMessage) -> Result<()> {
        let mut sent = self.sent.lock().map_err(|_| {
            NotificationError::new(
                ErrorKind::Transport,
                Some(Channel::Mail),
                "memory mail transport lock poisoned",
            )
        })?;
        if sent.len() >= self.capacity {
            return Err(NotificationError::new(
                ErrorKind::Capacity,
                Some(Channel::Mail),
                "memory mail transport capacity reached",
            ));
        }
        sent.push(message.clone());
        Ok(())
    }
}
