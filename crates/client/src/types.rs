use crate::{ClientError, ErrorKind, Result};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}
impl Method {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Header {
    name: String,
    value: String,
}
impl Header {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let value = value.into();
        if name.is_empty() || !name.bytes().all(is_token) {
            return Err(ClientError::new(
                ErrorKind::InvalidRequest,
                "HTTP header name is invalid",
            ));
        }
        if !value
            .bytes()
            .all(|byte| byte == b'\t' || (32..=126).contains(&byte))
        {
            return Err(ClientError::new(
                ErrorKind::InvalidRequest,
                "HTTP header value contains a prohibited byte",
            ));
        }
        Ok(Self {
            name: name.to_ascii_lowercase(),
            value,
        })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn value(&self) -> &str {
        &self.value
    }
}
impl fmt::Debug for Header {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Header")
            .field("name", &self.name)
            .field("value", &"[redacted]")
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Url {
    scheme: String,
    host: String,
    port: u16,
    path_and_query: String,
}
impl Url {
    pub fn parse(value: &str) -> Result<Self> {
        let (scheme, remainder) = value
            .split_once("://")
            .ok_or_else(|| ClientError::new(ErrorKind::InvalidUrl, "URL must include a scheme"))?;
        if scheme != "http" && scheme != "https" {
            return Err(ClientError::new(
                ErrorKind::UnsupportedScheme,
                "only http and https URL schemes are recognized",
            ));
        }
        if remainder.contains('#') {
            return Err(ClientError::new(
                ErrorKind::InvalidUrl,
                "URL fragments are not sent in HTTP requests",
            ));
        }
        let split = remainder.find(['/', '?']).unwrap_or(remainder.len());
        let authority = &remainder[..split];
        let suffix = &remainder[split..];
        if authority.is_empty()
            || authority.contains('@')
            || authority
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte == b' ')
        {
            return Err(ClientError::new(
                ErrorKind::InvalidUrl,
                "URL authority is invalid",
            ));
        }
        let (host, explicit_port) = parse_authority(authority)?;
        let port = explicit_port.unwrap_or(if scheme == "https" { 443 } else { 80 });
        if port == 0 {
            return Err(ClientError::new(
                ErrorKind::InvalidUrl,
                "URL port must be greater than zero",
            ));
        }
        let path_and_query = if suffix.is_empty() {
            "/".into()
        } else if suffix.starts_with('?') {
            format!("/{suffix}")
        } else {
            suffix.into()
        };
        if path_and_query
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b' ')
        {
            return Err(ClientError::new(
                ErrorKind::InvalidUrl,
                "URL path contains an invalid byte",
            ));
        }
        Ok(Self {
            scheme: scheme.into(),
            host,
            port,
            path_and_query,
        })
    }
    pub fn scheme(&self) -> &str {
        &self.scheme
    }
    pub fn host(&self) -> &str {
        &self.host
    }
    pub const fn port(&self) -> u16 {
        self.port
    }
    pub fn path_and_query(&self) -> &str {
        &self.path_and_query
    }
    pub(crate) fn authority(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        if (self.scheme == "http" && self.port == 80)
            || (self.scheme == "https" && self.port == 443)
        {
            host
        } else {
            format!("{host}:{}", self.port)
        }
    }
}
impl fmt::Debug for Url {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Url")
            .field("scheme", &self.scheme)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("path_and_query", &"[redacted]")
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Request {
    method: Method,
    url: Url,
    headers: Vec<Header>,
    body: Vec<u8>,
}
impl Request {
    pub fn new(method: Method, url: Url) -> Self {
        Self {
            method,
            url,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Result<Self> {
        self.headers.push(Header::new(name, value)?);
        Ok(self)
    }
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
    pub const fn method(&self) -> Method {
        self.method
    }
    pub fn url(&self) -> &Url {
        &self.url
    }
    pub fn headers(&self) -> &[Header] {
        &self.headers
    }
    pub fn body_bytes(&self) -> &[u8] {
        &self.body
    }
}
impl fmt::Debug for Request {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Request")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("header_count", &self.headers.len())
            .field("body_bytes", &self.body.len())
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Response {
    status: u16,
    headers: Vec<Header>,
    body: Vec<u8>,
}
impl Response {
    pub fn new(status: u16, headers: Vec<Header>, body: Vec<u8>) -> Result<Self> {
        if !(100..=599).contains(&status) {
            return Err(ClientError::new(
                ErrorKind::MalformedResponse,
                "response status is outside the HTTP range",
            ));
        }
        Ok(Self {
            status,
            headers,
            body,
        })
    }
    pub const fn status(&self) -> u16 {
        self.status
    }
    pub fn headers(&self) -> &[Header] {
        &self.headers
    }
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(Header::value)
    }
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    pub fn text(&self) -> Result<&str> {
        std::str::from_utf8(&self.body).map_err(|_| {
            ClientError::new(
                ErrorKind::MalformedResponse,
                "response body is not valid UTF-8",
            )
        })
    }
}
impl fmt::Debug for Response {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Response")
            .field("status", &self.status)
            .field("header_count", &self.headers.len())
            .field("body_bytes", &self.body.len())
            .finish()
    }
}

pub trait HttpClient: Send + Sync {
    fn send(&self, request: Request) -> Result<Response>;
}

fn parse_authority(authority: &str) -> Result<(String, Option<u16>)> {
    if let Some(rest) = authority.strip_prefix('[') {
        let end = rest.find(']').ok_or_else(|| {
            ClientError::new(
                ErrorKind::InvalidUrl,
                "IPv6 host is missing its closing bracket",
            )
        })?;
        let host = &rest[..end];
        let after = &rest[end + 1..];
        if host.is_empty() {
            return Err(ClientError::new(ErrorKind::InvalidUrl, "URL host is empty"));
        }
        let port = if after.is_empty() {
            None
        } else {
            Some(
                after
                    .strip_prefix(':')
                    .ok_or_else(|| {
                        ClientError::new(
                            ErrorKind::InvalidUrl,
                            "invalid characters after IPv6 host",
                        )
                    })?
                    .parse()
                    .map_err(|_| ClientError::new(ErrorKind::InvalidUrl, "URL port is invalid"))?,
            )
        };
        return Ok((host.into(), port));
    }
    let (host, port) =
        match authority.rsplit_once(':') {
            Some((host, port))
                if !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()) =>
            {
                (
                    host,
                    Some(port.parse().map_err(|_| {
                        ClientError::new(ErrorKind::InvalidUrl, "URL port is invalid")
                    })?),
                )
            }
            _ => (authority, None),
        };
    if host.is_empty() || host.contains(':') {
        return Err(ClientError::new(
            ErrorKind::InvalidUrl,
            "URL host is invalid",
        ));
    }
    Ok((host.into(), port))
}
fn is_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}
