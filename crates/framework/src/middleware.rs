use crate::{Request, Response, Result};
use std::{fmt, sync::Arc};

/// A middleware function can continue once or return early.
pub trait Middleware: Send + Sync + 'static {
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response>;
}
impl<F> Middleware for F
where
    F: for<'a> Fn(Request, Next<'a>) -> Result<Response> + Send + Sync + 'static,
{
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {
        self(request, next)
    }
}

pub struct Next<'a> {
    remaining: &'a [Arc<dyn Middleware>],
    terminal: &'a (dyn Fn(Request) -> Result<Response> + Send + Sync),
}
impl Next<'_> {
    pub fn run(self, request: Request) -> Result<Response> {
        match self.remaining.split_first() {
            Some((layer, rest)) => layer.handle(
                request,
                Next {
                    remaining: rest,
                    terminal: self.terminal,
                },
            ),
            None => (self.terminal)(request),
        }
    }
}
#[derive(Clone, Default)]
pub(crate) struct Layers(pub(crate) Vec<Arc<dyn Middleware>>);
impl fmt::Debug for Layers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Layers").field(&self.0.len()).finish()
    }
}
impl Layers {
    pub(crate) fn run(
        &self,
        request: Request,
        terminal: &(dyn Fn(Request) -> Result<Response> + Send + Sync),
    ) -> Result<Response> {
        Next {
            remaining: &self.0,
            terminal,
        }
        .run(request)
    }
}

/// Process-local diagnostic counter, not a security token or globally unique ID.
#[derive(Default)]
pub struct RequestId;
static IDS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
impl Middleware for RequestId {
    fn handle(&self, mut request: Request, next: Next<'_>) -> Result<Response> {
        let id = IDS
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .to_string();
        request.set_request_id(id.clone());
        next.run(request)?.header("x-request-id", &id)
    }
}

/// Requires a valid bearer credential and attaches its principal to the request.
#[cfg(feature = "auth")]
pub struct Authenticated<G> {
    guard: G,
}

#[cfg(feature = "auth")]
impl<G> Authenticated<G> {
    pub fn new(guard: G) -> Self {
        Self { guard }
    }
}

#[cfg(feature = "auth")]
impl<G: berserk_auth::Guard> Middleware for Authenticated<G> {
    fn handle(&self, mut request: Request, next: Next<'_>) -> Result<Response> {
        let token = request.header("authorization").and_then(bearer_token);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| {
                berserk_auth::AuthError::new(
                    berserk_auth::ErrorKind::Configuration,
                    "system clock is before the Unix epoch",
                )
            })?
            .as_secs();
        let principal = match token {
            Some(token) => self.guard.authenticate(token, now)?,
            None => None,
        };
        match principal {
            Some(principal) => {
                request.set_principal(principal);
                next.run(request)
            }
            None => Response::text("Unauthorized")
                .status(401)
                .header("www-authenticate", "Bearer"),
        }
    }
}

#[cfg(feature = "auth")]
fn bearer_token(value: &str) -> Option<&str> {
    let mut parts = value.split_ascii_whitespace();
    let scheme = parts.next()?;
    let token = parts.next()?;
    if !scheme.eq_ignore_ascii_case("Bearer") || token.is_empty() || parts.next().is_some() {
        return None;
    }
    Some(token)
}
