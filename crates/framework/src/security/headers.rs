use crate::{ConfigError, Headers, Middleware, Next, Request, Response, Result};
use std::time::Duration;

/// Conservative response headers for APIs, including downstream error responses.
/// Defaults: nosniff, frame denial, and no referrer disclosure. CSP, HSTS, and
/// Permissions-Policy are opt-in because they depend on application/deployment policy.
#[derive(Clone, Debug)]
pub struct SecurityHeaders {
    headers: Headers,
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        let mut headers = Headers::new();
        for (name, value) in [
            ("x-content-type-options", "nosniff"),
            ("x-frame-options", "DENY"),
            ("referrer-policy", "no-referrer"),
        ] {
            headers.insert(name, value).expect("static security header");
        }
        Self { headers }
    }
}

impl SecurityHeaders {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set DENY or SAMEORIGIN. Use CSP frame-ancestors for a more specific policy.
    pub fn frame_options(self, value: &str) -> Result<Self> {
        if !matches!(value, "DENY" | "SAMEORIGIN") {
            return Err(config("frame options must be DENY or SAMEORIGIN"));
        }
        self.header("x-frame-options", value)
    }

    pub fn referrer_policy(self, value: &str) -> Result<Self> {
        if !matches!(
            value,
            "no-referrer"
                | "no-referrer-when-downgrade"
                | "same-origin"
                | "origin"
                | "strict-origin"
                | "origin-when-cross-origin"
                | "strict-origin-when-cross-origin"
                | "unsafe-url"
        ) {
            return Err(config("invalid referrer policy"));
        }
        self.header("referrer-policy", value)
    }

    /// Validate header syntax; applications remain responsible for CSP semantics.
    pub fn content_security_policy(self, value: &str) -> Result<Self> {
        self.header("content-security-policy", value)
    }

    /// Validate header syntax; applications remain responsible for policy semantics.
    pub fn permissions_policy(self, value: &str) -> Result<Self> {
        self.header("permissions-policy", value)
    }

    /// Enable only for HTTPS deployments. No inference is made from proxy headers.
    /// Whole seconds are required; zero removes a browser's existing HSTS policy.
    /// Preload is deliberately not enabled.
    pub fn hsts(self, age: Duration, include_subdomains: bool) -> Result<Self> {
        if age.subsec_nanos() != 0 || age.as_secs() > u32::MAX as u64 {
            return Err(config("HSTS max age must be whole seconds within u32::MAX"));
        }
        let value = format!(
            "max-age={}{}",
            age.as_secs(),
            if include_subdomains {
                "; includeSubDomains"
            } else {
                ""
            }
        );
        self.header("strict-transport-security", &value)
    }

    fn header(mut self, name: &str, value: &str) -> Result<Self> {
        if value.trim().is_empty() || value.len() > 8192 {
            return Err(config("security header must contain 1 to 8192 bytes"));
        }
        self.headers
            .insert(name, value)
            .map_err(|_| config("invalid security header value"))?;
        Ok(self)
    }
}

impl Middleware for SecurityHeaders {
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {
        let mut response = next.run(request).unwrap_or_else(|error| error.response());
        for (name, value) in self.headers.iter() {
            response = response.header(name, value)?;
        }
        Ok(response)
    }
}

fn config(message: &'static str) -> crate::Error {
    ConfigError::new("security_headers", message).into()
}
