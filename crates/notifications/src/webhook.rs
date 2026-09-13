use crate::{Channel, ErrorKind, NotificationError, Result};
use framework_client::{Header, Method, Request, Url};
use std::fmt;

#[derive(Clone, Eq, PartialEq)]
pub struct WebhookMessage {
    url: Url,
    headers: Vec<Header>,
    body: Vec<u8>,
    idempotency_key: Option<String>,
}
impl WebhookMessage {
    pub fn json(url: Url, body: impl Into<Vec<u8>>) -> Result<Self> {
        let body = body.into();
        if body.is_empty() {
            return Err(NotificationError::new(
                ErrorKind::InvalidMessage,
                Some(Channel::Webhook),
                "webhook body cannot be empty",
            ));
        }
        Ok(Self {
            url,
            headers: vec![Header::new("content-type", "application/json").map_err(client_error)?],
            body,
            idempotency_key: None,
        })
    }
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Result<Self> {
        self.headers
            .push(Header::new(name, value).map_err(client_error)?);
        Ok(self)
    }
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Result<Self> {
        let key = key.into();
        if key.is_empty() || key.len() > 256 || key.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(NotificationError::new(
                ErrorKind::InvalidMessage,
                Some(Channel::Webhook),
                "webhook idempotency key is invalid",
            ));
        }
        self.idempotency_key = Some(key);
        Ok(self)
    }
    pub fn request(&self) -> Result<Request> {
        let mut request = Request::new(Method::Post, self.url.clone()).body(self.body.clone());
        for header in &self.headers {
            request = request
                .header(header.name(), header.value())
                .map_err(client_error)?;
        }
        if let Some(key) = &self.idempotency_key {
            request = request
                .header("idempotency-key", key)
                .map_err(client_error)?;
        }
        Ok(request)
    }
    pub fn body_bytes(&self) -> usize {
        self.body.len()
    }
    pub fn url(&self) -> &Url {
        &self.url
    }
}
impl fmt::Debug for WebhookMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WebhookMessage")
            .field("url", &self.url)
            .field("header_count", &self.headers.len())
            .field("body_bytes", &self.body.len())
            .field("has_idempotency_key", &self.idempotency_key.is_some())
            .finish()
    }
}
fn client_error(error: framework_client::ClientError) -> NotificationError {
    NotificationError::new(
        ErrorKind::InvalidMessage,
        Some(Channel::Webhook),
        error.to_string(),
    )
}
