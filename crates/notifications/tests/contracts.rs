use berserk_client::{ClientError, HttpClient, Response, Result as ClientResult, Url};
use berserk_notifications::{
    Channel, DeliveryOutcome, DeliveryPolicy, EmailAddress, MailMessage, MemoryMailTransport,
    Notification, Notifier, NotifierConfig, Recipient, WebhookMessage,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Welcome {
    from: EmailAddress,
}
impl Notification for Welcome {
    fn mail(&self, recipient: &Recipient) -> berserk_notifications::Result<Option<MailMessage>> {
        recipient
            .email_address()
            .map(|to| MailMessage::text(self.from.clone(), to.clone(), "Welcome", "Hello"))
            .transpose()
    }
    fn webhook(
        &self,
        recipient: &Recipient,
    ) -> berserk_notifications::Result<Option<WebhookMessage>> {
        recipient
            .webhook_url()
            .map(|url| {
                WebhookMessage::json(url.clone(), br#"{"event":"welcome"}"#.to_vec())
                    .and_then(|message| message.idempotency_key("welcome-1"))
            })
            .transpose()
    }
}

#[test]
fn mail_and_webhook_channels_report_success() {
    let mail = Arc::new(MemoryMailTransport::new(4).unwrap());
    let http = Arc::new(SequenceClient::new(vec![200]));
    let notifier = Notifier::new(NotifierConfig::default())
        .unwrap()
        .with_mail(mail.clone())
        .with_http(http.clone());
    let recipient = Recipient::new()
        .email(EmailAddress::new("user@example.com").unwrap())
        .webhook(Url::parse("https://hooks.example.com/secret").unwrap());
    let report = notifier
        .send(
            &recipient,
            &Welcome {
                from: EmailAddress::new("app@example.com").unwrap(),
            },
        )
        .unwrap();
    assert!(report.all_sent());
    assert_eq!(report.outcomes.len(), 2);
    assert_eq!(mail.sent().unwrap().len(), 1);
    assert_eq!(http.calls(), 1);
}

#[test]
fn retryable_webhook_statuses_retry_but_client_errors_do_not() {
    let retry = NotifierConfig {
        delivery: DeliveryPolicy {
            max_attempts: 3,
            backoff: Duration::ZERO,
        },
        ..NotifierConfig::default()
    };
    let http = Arc::new(SequenceClient::new(vec![503, 200]));
    let notifier = Notifier::new(retry).unwrap().with_http(http.clone());
    let recipient =
        Recipient::new().webhook(Url::parse("https://hooks.example.com/secret").unwrap());
    let report = notifier
        .send(
            &recipient,
            &Welcome {
                from: EmailAddress::new("app@example.com").unwrap(),
            },
        )
        .unwrap();
    assert_eq!(
        report.outcomes,
        vec![DeliveryOutcome::Sent {
            channel: Channel::Webhook,
            attempts: 2
        }]
    );
    let terminal = Arc::new(SequenceClient::new(vec![400, 200]));
    let notifier = Notifier::new(retry).unwrap().with_http(terminal.clone());
    let report = notifier
        .send(
            &recipient,
            &Welcome {
                from: EmailAddress::new("app@example.com").unwrap(),
            },
        )
        .unwrap();
    assert!(matches!(
        &report.outcomes[0],
        DeliveryOutcome::Failed { attempts: 1, .. }
    ));
    assert_eq!(terminal.calls(), 1);
}

#[test]
fn missing_transports_and_header_injection_are_explicit() {
    assert!(EmailAddress::new("bad\r\n@example.com").is_err());
    let notifier = Notifier::new(NotifierConfig::default()).unwrap();
    let recipient = Recipient::new().email(EmailAddress::new("user@example.com").unwrap());
    let report = notifier
        .send(
            &recipient,
            &Welcome {
                from: EmailAddress::new("app@example.com").unwrap(),
            },
        )
        .unwrap();
    assert!(matches!(
        &report.outcomes[0],
        DeliveryOutcome::Failed {
            channel: Channel::Mail,
            attempts: 0,
            ..
        }
    ));
}

struct SequenceClient {
    statuses: Mutex<Vec<u16>>,
    calls: Mutex<usize>,
}
impl SequenceClient {
    fn new(mut statuses: Vec<u16>) -> Self {
        statuses.reverse();
        Self {
            statuses: Mutex::new(statuses),
            calls: Mutex::new(0),
        }
    }
    fn calls(&self) -> usize {
        *self.calls.lock().unwrap()
    }
}
impl HttpClient for SequenceClient {
    fn send(&self, _request: berserk_client::Request) -> ClientResult<Response> {
        *self.calls.lock().unwrap() += 1;
        let status =
            self.statuses.lock().unwrap().pop().ok_or_else(|| {
                ClientError::new(berserk_client::ErrorKind::Transport, "no response")
            })?;
        Response::new(status, Vec::new(), Vec::new())
    }
}
