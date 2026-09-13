use super::{Headers, HttpError, StatusCode};

#[derive(Clone)]
pub struct Response {
    status: u16,
    headers: Headers,
    body: Vec<u8>,
    head_length: Option<usize>,
    pub(crate) stream: Option<StreamBody>,
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("header_count", &self.headers.len())
            .field("body_bytes", &self.body.len())
            .field("streamed", &self.stream.is_some())
            .finish()
    }
}

impl Response {
    pub fn empty() -> Self {
        Self {
            status: 200,
            headers: Headers::new(),
            body: Vec::new(),
            head_length: None,
            stream: None,
        }
    }

    /// One-shot stream; cloned responses share consumption. HEAD never reads the source.
    pub fn stream(reader: impl std::io::Read + Send + 'static) -> Self {
        let mut response = Self::empty();
        response.stream = Some(StreamBody(std::sync::Arc::new(std::sync::Mutex::new(
            Some(Box::new(reader)),
        ))));
        response
    }
    pub fn text(value: impl Into<String>) -> Self {
        let mut response = Self::empty();
        response.body = value.into().into_bytes();
        response
            .headers
            .insert("content-type", "text/plain; charset=utf-8")
            .expect("static header is valid");
        response
    }

    pub fn bytes(value: impl Into<Vec<u8>>) -> Self {
        let mut response = Self::empty();
        response.body = value.into();
        response
            .headers
            .insert("content-type", "application/octet-stream")
            .expect("static header is valid");
        response
    }

    /// Builder remains infallible; validate before any bytes are encoded.
    pub fn status(mut self, code: u16) -> Self {
        self.status = code;
        self
    }
    pub fn status_code(&self) -> u16 {
        self.status
    }
    pub fn headers(&self) -> &Headers {
        &self.headers
    }
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    /// Pre-suppression body size for HEAD, otherwise current body byte length.
    /// The future encoder must still apply status-specific framing rules.
    pub fn representation_length(&self) -> usize {
        self.head_length.unwrap_or(self.body.len())
    }
    pub(crate) fn suppress_for_head(&mut self) {
        if self.head_length.is_none() {
            self.head_length = Some(self.body.len());
        }
        self.body.clear();
    }

    fn check_header(name: &str) -> Result<(), HttpError> {
        if ["content-length", "transfer-encoding", "connection"]
            .iter()
            .any(|reserved| name.eq_ignore_ascii_case(reserved))
        {
            return Err(HttpError::ReservedResponseHeader);
        }
        Ok(())
    }

    pub fn header(mut self, name: &str, value: &str) -> crate::Result<Self> {
        Self::check_header(name)?;
        self.headers.insert(name, value)?;
        Ok(self)
    }

    pub fn append_header(mut self, name: &str, value: &str) -> crate::Result<Self> {
        Self::check_header(name)?;
        self.headers.append(name, value)?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), HttpError> {
        let status = StatusCode::new(self.status)?;
        if !status.allows_body() && (!self.body.is_empty() || self.stream.is_some()) {
            return Err(HttpError::BodyNotAllowed(self.status));
        }
        Ok(())
    }
}

/// Preserve errors until the server decides how to log and render them.
pub trait IntoResponse {
    fn into_response(self) -> crate::Result<Response>;
}
impl IntoResponse for Response {
    fn into_response(self) -> crate::Result<Response> {
        self.validate()?;
        Ok(self)
    }
}
impl IntoResponse for crate::Result<Response> {
    fn into_response(self) -> crate::Result<Response> {
        self?.into_response()
    }
}

#[derive(Clone)]
pub(crate) struct StreamBody(
    pub(crate) std::sync::Arc<std::sync::Mutex<Option<Box<dyn std::io::Read + Send>>>>,
);
impl std::fmt::Debug for StreamBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StreamBody(..)")
    }
}
