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

    pub fn html(value: impl Into<String>) -> Self {
        let mut response = Self::empty();
        response.body = value.into().into_bytes();
        response
            .headers
            .insert("content-type", "text/html; charset=utf-8")
            .expect("static header is valid");
        response
    }

    #[cfg(feature = "view")]
    pub fn view<D>(view: &str, data: D) -> crate::Result<Self>
    where
        D: Into<berserk_axe::Context>,
    {
        let context = data.into();
        Ok(Self::html(berserk_axe::render(view, &context)?))
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

    #[cfg(feature = "server")]
    pub(crate) fn take_body(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.body)
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

    /// Remove all values for a header, for example before enforcing a CORS policy.
    pub fn without_header(mut self, name: &str) -> Self {
        self.headers.remove(name);
        self
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

impl<T: IntoResponse> IntoResponse for crate::Result<T> {
    fn into_response(self) -> crate::Result<Response> {
        self?.into_response()
    }
}

#[derive(Clone)]
pub(crate) struct StreamBody(
    // Only the optional network transport consumes the stored reader.
    #[cfg_attr(not(feature = "server"), allow(dead_code))]
    pub(crate)  std::sync::Arc<std::sync::Mutex<Option<Box<dyn std::io::Read + Send>>>>,
);

impl std::fmt::Debug for StreamBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StreamBody(..)")
    }
}

impl IntoResponse for crate::Json {
    fn into_response(self) -> crate::Result<Response> {
        Response::json(&self)
    }
}
impl IntoResponse for String {
    fn into_response(self) -> crate::Result<Response> {
        Ok(Response::text(self))
    }
}
impl IntoResponse for &str {
    fn into_response(self) -> crate::Result<Response> {
        Ok(Response::text(self))
    }
}
impl IntoResponse for () {
    fn into_response(self) -> crate::Result<Response> {
        Ok(Response::no_content())
    }
}
impl Response {
    pub fn no_content() -> Self {
        Self::empty().status(204)
    }
    pub fn created(value: &crate::Json) -> crate::Result<Self> {
        Ok(Self::json(value)?.status(201))
    }
}
