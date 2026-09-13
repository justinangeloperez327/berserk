use crate::{
    Channel, EmailAddress, ErrorKind, MailMessage, MailTransport, NotificationError, Result,
    WebhookMessage,
};
use framework_client::{ErrorKind as ClientErrorKind, HttpClient};
use std::{sync::Arc, thread, time::Duration};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Recipient {
    email: Option<EmailAddress>,
    webhook: Option<framework_client::Url>,
}
impl Recipient {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn email(mut self, email: EmailAddress) -> Self {
        self.email = Some(email);
        self
    }
    pub fn webhook(mut self, webhook: framework_client::Url) -> Self {
        self.webhook = Some(webhook);
        self
    }
    pub fn email_address(&self) -> Option<&EmailAddress> {
        self.email.as_ref()
    }
    pub fn webhook_url(&self) -> Option<&framework_client::Url> {
        self.webhook.as_ref()
    }
}

pub trait Notification {
    fn mail(&self, _recipient: &Recipient) -> Result<Option<MailMessage>> {
        Ok(None)
    }
    fn webhook(&self, _recipient: &Recipient) -> Result<Option<WebhookMessage>> {
        Ok(None)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeliveryPolicy {
    pub max_attempts: u32,
    pub backoff: Duration,
}
impl DeliveryPolicy {
    pub fn validate(self) -> Result<Self> {
        if self.max_attempts == 0 {
            Err(NotificationError::new(
                ErrorKind::Configuration,
                None,
                "delivery attempts must be greater than zero",
            ))
        } else {
            Ok(self)
        }
    }
}
impl Default for DeliveryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            backoff: Duration::ZERO,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifierConfig {
    pub delivery: DeliveryPolicy,
    pub max_mail_body_bytes: usize,
    pub max_webhook_body_bytes: usize,
    pub allow_insecure_webhooks: bool,
}
impl Default for NotifierConfig {
    fn default() -> Self {
        Self {
            delivery: DeliveryPolicy::default(),
            max_mail_body_bytes: 1024 * 1024,
            max_webhook_body_bytes: 1024 * 1024,
            allow_insecure_webhooks: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeliveryOutcome {
    Sent {
        channel: Channel,
        attempts: u32,
    },
    Failed {
        channel: Channel,
        attempts: u32,
        error: String,
    },
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DeliveryReport {
    pub outcomes: Vec<DeliveryOutcome>,
}
impl DeliveryReport {
    pub fn all_sent(&self) -> bool {
        self.outcomes
            .iter()
            .all(|outcome| matches!(outcome, DeliveryOutcome::Sent { .. }))
    }
}

pub struct Notifier {
    config: NotifierConfig,
    mail: Option<Arc<dyn MailTransport>>,
    http: Option<Arc<dyn HttpClient>>,
}
impl Notifier {
    pub fn new(config: NotifierConfig) -> Result<Self> {
        config.delivery.validate()?;
        if config.max_mail_body_bytes == 0 || config.max_webhook_body_bytes == 0 {
            return Err(NotificationError::new(
                ErrorKind::Configuration,
                None,
                "notification body limits must be greater than zero",
            ));
        }
        Ok(Self {
            config,
            mail: None,
            http: None,
        })
    }
    pub fn with_mail(mut self, transport: Arc<dyn MailTransport>) -> Self {
        self.mail = Some(transport);
        self
    }
    pub fn with_http(mut self, client: Arc<dyn HttpClient>) -> Self {
        self.http = Some(client);
        self
    }
    pub fn send<N: Notification>(
        &self,
        recipient: &Recipient,
        notification: &N,
    ) -> Result<DeliveryReport> {
        let mut report = DeliveryReport::default();
        if let Some(message) = notification.mail(recipient)? {
            report.outcomes.push(self.deliver_mail(&message));
        }
        if let Some(message) = notification.webhook(recipient)? {
            report.outcomes.push(self.deliver_webhook(&message));
        }
        Ok(report)
    }
    fn deliver_mail(&self, message: &MailMessage) -> DeliveryOutcome {
        let Some(transport) = &self.mail else {
            return failed(Channel::Mail, 0, "mail transport is not configured");
        };
        if message.body_bytes() > self.config.max_mail_body_bytes {
            return failed(Channel::Mail, 0, "mail body exceeds configured byte limit");
        }
        retry(self.config.delivery, Channel::Mail, || {
            transport.send(message).map_err(|error| AttemptFailure {
                message: error.to_string(),
                retryable: error.kind() == ErrorKind::Transport,
            })
        })
    }
    fn deliver_webhook(&self, message: &WebhookMessage) -> DeliveryOutcome {
        let Some(client) = &self.http else {
            return failed(Channel::Webhook, 0, "HTTP transport is not configured");
        };
        if message.body_bytes() > self.config.max_webhook_body_bytes {
            return failed(
                Channel::Webhook,
                0,
                "webhook body exceeds configured byte limit",
            );
        }
        if !self.config.allow_insecure_webhooks && message.url().scheme() != "https" {
            return failed(Channel::Webhook, 0, "webhook URL must use https");
        }
        retry(self.config.delivery, Channel::Webhook, || {
            let request = message
                .request()
                .map_err(|error| AttemptFailure::terminal(error.to_string()))?;
            let response = client.send(request).map_err(|error| {
                let retryable = matches!(
                    error.kind(),
                    ClientErrorKind::Dns
                        | ClientErrorKind::Connect
                        | ClientErrorKind::Timeout
                        | ClientErrorKind::Write
                        | ClientErrorKind::Io
                        | ClientErrorKind::Transport
                );
                AttemptFailure {
                    message: error.to_string(),
                    retryable,
                }
            })?;
            if (200..300).contains(&response.status()) {
                Ok(())
            } else {
                let status = response.status();
                Err(AttemptFailure {
                    message: format!("webhook endpoint returned HTTP {status}"),
                    retryable: status == 408 || status == 429 || status >= 500,
                })
            }
        })
    }
}
struct AttemptFailure {
    message: String,
    retryable: bool,
}
impl AttemptFailure {
    fn terminal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
        }
    }
}
fn retry<F>(policy: DeliveryPolicy, channel: Channel, mut operation: F) -> DeliveryOutcome
where
    F: FnMut() -> std::result::Result<(), AttemptFailure>,
{
    let mut final_error = String::new();
    for attempt in 1..=policy.max_attempts {
        match operation() {
            Ok(()) => {
                return DeliveryOutcome::Sent {
                    channel,
                    attempts: attempt,
                }
            }
            Err(error) => {
                final_error = error.message;
                if !error.retryable {
                    return failed(channel, attempt, final_error);
                }
            }
        }
        if attempt < policy.max_attempts && !policy.backoff.is_zero() {
            thread::sleep(policy.backoff);
        }
    }
    failed(channel, policy.max_attempts, final_error)
}
fn failed(channel: Channel, attempts: u32, error: impl Into<String>) -> DeliveryOutcome {
    DeliveryOutcome::Failed {
        channel,
        attempts,
        error: error.into(),
    }
}
