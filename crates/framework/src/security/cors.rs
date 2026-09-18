use crate::{ConfigError, Headers, Method, Middleware, Next, Request, Response, Result};
use std::{collections::BTreeSet, net::Ipv6Addr, time::Duration};

const MAX_ITEMS: usize = 128;
const CORS_HEADERS: [&str; 6] = [
    "access-control-allow-origin",
    "access-control-allow-credentials",
    "access-control-allow-methods",
    "access-control-allow-headers",
    "access-control-expose-headers",
    "access-control-max-age",
];

/// An explicit, application-wide CORS policy. Register before authentication.
/// Defaults allow no origins, GET/HEAD methods, no request headers, and no credentials.
/// CORS controls browser response access; it does not authenticate requests or prevent CSRF.
#[derive(Clone, Debug)]
pub struct Cors {
    origins: BTreeSet<String>,
    any_origin: bool,
    methods: BTreeSet<String>,
    headers: BTreeSet<String>,
    exposed: BTreeSet<String>,
    credentials: bool,
    max_age: Option<u32>,
}

impl Default for Cors {
    fn default() -> Self {
        Self {
            origins: BTreeSet::new(),
            any_origin: false,
            methods: ["GET".to_owned(), "HEAD".to_owned()].into(),
            headers: BTreeSet::new(),
            exposed: BTreeSet::new(),
            credentials: false,
            max_age: None,
        }
    }
}

impl Cors {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an exact HTTP(S) origin, with no path, credentials, query, or fragment.
    /// Use lower-case ASCII host names (IDNA names must already be punycode).
    /// Opaque `null` origins and subdomain wildcards cannot be explicitly trusted.
    pub fn allow_origin(mut self, origin: &str) -> Result<Self> {
        if origin == "*" {
            if self.credentials || !self.origins.is_empty() {
                return Err(config("wildcard origins cannot be mixed with credentials or exact origins"));
            }
            self.any_origin = true;
        } else {
            if self.any_origin || !valid_origin(origin) {
                return Err(config("expected an exact serialized HTTP(S) origin"));
            }
            add(&mut self.origins, origin.to_owned())?;
        }
        Ok(self)
    }

    /// Replace the allowed method list. Method names are case-sensitive.
    pub fn allow_methods<I, S>(mut self, methods: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut allowed = BTreeSet::new();
        for method in methods {
            let method = method.as_ref();
            if !valid_method(method) {
                return Err(config("invalid or forbidden CORS method"));
            }
            add(&mut allowed, method.to_owned())?;
        }
        if allowed.is_empty() {
            return Err(config("at least one CORS method is required"));
        }
        self.methods = allowed;
        Ok(self)
    }

    /// Replace the non-safelisted request-header allowlist. Wildcards are not supported.
    pub fn allow_headers<I, S>(mut self, headers: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.headers = header_list(headers)?;
        Ok(self)
    }

    /// Replace the headers browsers may expose to application code.
    pub fn expose_headers<I, S>(mut self, headers: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.exposed = header_list(headers)?;
        if self.exposed.contains("set-cookie") || self.exposed.contains("set-cookie2") {
            return Err(config("cookie headers cannot be exposed to browser scripts"));
        }
        Ok(self)
    }

    /// Allow credentialed browser requests only for explicitly trusted origins.
    pub fn credentials(mut self, enabled: bool) -> Result<Self> {
        if enabled && self.any_origin {
            return Err(config("credentials require exact allowed origins"));
        }
        self.credentials = enabled;
        Ok(self)
    }

    /// Set the preflight cache lifetime in whole seconds (at most u32::MAX).
    /// Browsers may impose a smaller limit. Zero disables preflight caching.
    pub fn max_age(mut self, age: Duration) -> Result<Self> {
        if age.subsec_nanos() != 0 {
            return Err(config("CORS max age must use whole seconds"));
        }
        self.max_age = Some(u32::try_from(age.as_secs()).map_err(|_| config("CORS max age is too large"))?);
        Ok(self)
    }

    fn check(&self, request: &Request) -> Result<Option<(String, bool)>> {
        let origin = single(request.headers(), "origin")?;
        let method = single(request.headers(), "access-control-request-method")?;
        let headers = single(request.headers(), "access-control-request-headers")?;
        let preflight = request.method().as_str() == "OPTIONS" && method.is_some();
        if (method.is_some() || headers.is_some()) && (!preflight || origin.is_none()) {
            return Err(crate::Error::bad_request("Malformed CORS preflight"));
        }
        let Some(origin) = origin else {
            return Ok(None);
        };
        if origin != "null" && !valid_origin(origin) {
            return Err(crate::Error::bad_request("Malformed Origin"));
        }
        if !self.any_origin && !self.origins.contains(origin) {
            return Err(crate::Error::forbidden());
        }
        let requested_method = method.unwrap_or_else(|| request.method().as_str());
        if !valid_method(requested_method) {
            return Err(crate::Error::bad_request("Malformed CORS method"));
        }
        if !self.methods.contains(requested_method) {
            return Err(crate::Error::forbidden());
        }
        if let Some(headers) = headers {
            if headers.len() > 8192 {
                return Err(crate::Error::bad_request("CORS header list is too large"));
            }
            for (index, name) in headers.split(',').enumerate() {
                let name = name.trim_matches([' ', '\t']);
                if index >= MAX_ITEMS || !valid_header(name) {
                    return Err(crate::Error::bad_request("Malformed CORS header list"));
                }
                if !self.headers.contains(&name.to_ascii_lowercase()) {
                    return Err(crate::Error::forbidden());
                }
            }
        }
        Ok(Some((if self.any_origin { "*" } else { origin }.to_owned(), preflight)))
    }

    fn decorate(&self, mut response: Response, origin: &str, preflight: bool) -> Result<Response> {
        response = response.header("access-control-allow-origin", origin)?;
        if self.credentials {
            response = response.header("access-control-allow-credentials", "true")?;
        }
        if preflight {
            response = response.header("access-control-allow-methods", &joined(&self.methods))?;
            if !self.headers.is_empty() {
                response = response.header("access-control-allow-headers", &joined(&self.headers))?;
            }
            if let Some(age) = self.max_age {
                response = response.header("access-control-max-age", &age.to_string())?;
            }
        } else if !self.exposed.is_empty() {
            response = response.header("access-control-expose-headers", &joined(&self.exposed))?;
        }
        Ok(response)
    }
}

impl Middleware for Cors {
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {
        let preflight = request.method().as_str() == "OPTIONS";
        let decision = self.check(&request);
        let mut response = match &decision {
            Err(error) => error.response(),
            Ok(Some((_, true))) => Response::no_content(),
            _ => next.run(request).unwrap_or_else(|error| error.response()),
        };
        // Enforce one policy even if a handler supplies conflicting CORS headers.
        for header in CORS_HEADERS {
            response = response.without_header(header);
        }
        response = vary(response, preflight)?;
        if let Ok(Some((origin, preflight))) = decision {
            response = self.decorate(response, &origin, preflight)?;
        }
        Ok(response)
    }
}

fn config(message: &'static str) -> crate::Error {
    ConfigError::new("cors", message).into()
}

fn add(values: &mut BTreeSet<String>, value: String) -> Result<()> {
    if !values.contains(&value) && values.len() >= MAX_ITEMS {
        return Err(config("CORS lists are limited to 128 entries"));
    }
    values.insert(value);
    Ok(())
}

fn single<'a>(headers: &'a Headers, name: &'a str) -> Result<Option<&'a str>> {
    let mut values = headers.get_all(name);
    let first = values.next();
    if values.next().is_some() {
        return Err(crate::Error::bad_request("Duplicate CORS request header"));
    }
    Ok(first)
}

fn valid_method(method: &str) -> bool {
    method.len() <= 64
        && Method::new(method).is_ok()
        && !["*", "CONNECT", "TRACE", "TRACK"].iter().any(|name| method.eq_ignore_ascii_case(name))
}

fn valid_header(name: &str) -> bool {
    name.len() <= 128 && name != "*" && Headers::new().insert(name, "").is_ok()
}

fn header_list<I, S>(headers: I) -> Result<BTreeSet<String>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut allowed = BTreeSet::new();
    for name in headers {
        let name = name.as_ref();
        if !valid_header(name) {
            return Err(config("invalid CORS header name"));
        }
        add(&mut allowed, name.to_ascii_lowercase())?;
    }
    Ok(allowed)
}

fn joined(values: &BTreeSet<String>) -> String {
    values.iter().map(String::as_str).collect::<Vec<_>>().join(", ")
}

fn vary(response: Response, preflight: bool) -> Result<Response> {
    let mut values: Vec<String> = response.headers().get_all("vary")
        .flat_map(|value| value.split(','))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect();
    if values.iter().any(|value| value == "*") {
        return Ok(response);
    }
    let names: &[&str] = if preflight {
        &["Origin", "Access-Control-Request-Method", "Access-Control-Request-Headers"]
    } else {
        &["Origin"]
    };
    for name in names {
        if !values.iter().any(|value| value.eq_ignore_ascii_case(name)) {
            values.push((*name).to_owned());
        }
    }
    response.header("vary", &values.join(", "))
}

// Deliberately accept a narrow, unambiguous subset of serialized HTTP(S) origins.
// Never normalize an untrusted origin and then reflect a different spelling.
fn valid_origin(origin: &str) -> bool {
    if origin.len() > 2048 {
        return false;
    }
    let Some(authority) = origin.strip_prefix("https://").or_else(|| origin.strip_prefix("http://")) else {
        return false;
    };
    let (host, port) = if let Some(ipv6) = authority.strip_prefix('[') {
        let Some((address, rest)) = ipv6.split_once(']') else { return false; };
        if address.parse::<Ipv6Addr>().is_err() {
            return false;
        }
        (None, rest)
    } else {
        let (host, rest) = authority.find(':').map_or((authority, ""), |index| authority.split_at(index));
        (Some(host), rest)
    };
    if let Some(host) = host {
        if host.is_empty() || host.len() > 253 || !host.split('.').all(|label| {
            !label.is_empty() && label.len() <= 63
                && !label.starts_with('-') && !label.ends_with('-')
                && label.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        }) {
            return false;
        }
    }
    port.is_empty() || port.strip_prefix(':').is_some_and(|port| {
        !port.is_empty() && (port == "0" || !port.starts_with('0'))
            && port.bytes().all(|byte| byte.is_ascii_digit()) && port.parse::<u16>().is_ok()
    })
}
