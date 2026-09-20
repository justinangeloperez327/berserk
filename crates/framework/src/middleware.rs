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
        match authenticate_request(&request, &self.guard) {
            Ok(Some(principal)) => {
                request.set_principal(principal);
                next.run(request)
            }
            Ok(None) => Ok(crate::Error::unauthorized().response()),
            Err(error) if error.status_code() == 401 => Ok(error.response()),
            Err(error) => Err(error),
        }
    }
}

/// Allows guests and rejects authenticated identities with 403.
/// Invalid credentials are guests; malformed or ambiguous headers are rejected with 401.
#[cfg(feature = "auth")]
pub struct Guest<G> {
    guard: G,
}

#[cfg(feature = "auth")]
impl<G> Guest<G> {
    pub fn new(guard: G) -> Self {
        Self { guard }
    }
}

#[cfg(feature = "auth")]
impl<G: berserk_auth::Guard> Middleware for Guest<G> {
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {
        match authenticate_request(&request, &self.guard) {
            Ok(None) if request.user().is_none() => next.run(request),
            Ok(_) => Ok(crate::Error::forbidden().response()),
            Err(error) if error.status_code() == 401 => Ok(error.response()),
            Err(error) => Err(error),
        }
    }
}

#[cfg(feature = "auth")]
#[derive(Clone)]
pub(crate) struct ConfiguredAuth {
    guard: Arc<dyn berserk_auth::Guard>,
}

#[cfg(feature = "auth")]
impl ConfiguredAuth {
    pub(crate) fn new<G: berserk_auth::Guard>(guard: G) -> Self {
        Self {
            guard: Arc::new(guard),
        }
    }

    fn guard(&self) -> &dyn berserk_auth::Guard {
        self.guard.as_ref()
    }
}

#[cfg(feature = "auth")]
pub(crate) struct ConfiguredAuthenticated;

#[cfg(feature = "auth")]
impl Middleware for ConfiguredAuthenticated {
    fn handle(&self, mut request: Request, next: Next<'_>) -> Result<Response> {
        let outcome = {
            let auth = request.state::<ConfiguredAuth>().ok_or_else(|| {
                crate::ConfigError::new("auth", "authentication guard is not configured")
            })?;
            authenticate_request(&request, auth.guard())
        };

        match outcome {
            Ok(Some(principal)) => {
                request.set_principal(principal);
                next.run(request)
            }
            Ok(None) => Ok(crate::Error::unauthorized().response()),
            Err(error) if error.status_code() == 401 => Ok(error.response()),
            Err(error) => Err(error),
        }
    }
}

#[cfg(feature = "auth")]
pub(crate) struct ConfiguredGuest;

#[cfg(feature = "auth")]
impl Middleware for ConfiguredGuest {
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {
        let outcome = {
            let auth = request.state::<ConfiguredAuth>().ok_or_else(|| {
                crate::ConfigError::new("auth", "authentication guard is not configured")
            })?;
            authenticate_request(&request, auth.guard())
        };

        match outcome {
            Ok(None) if request.user().is_none() => next.run(request),
            Ok(_) => Ok(crate::Error::forbidden().response()),
            Err(error) if error.status_code() == 401 => Ok(error.response()),
            Err(error) => Err(error),
        }
    }
}

#[cfg(feature = "auth")]
fn authenticate_request<G: berserk_auth::Guard + ?Sized>(
    request: &Request,
    guard: &G,
) -> Result<Option<berserk_auth::Principal>> {
    let mut headers = request.headers().get_all("authorization");
    let first = headers.next();
    if headers.next().is_some() {
        return Err(crate::Error::unauthorized());
    }
    let Some(header) = first else {
        return Ok(None);
    };
    let token = bearer_token(header).ok_or_else(crate::Error::unauthorized)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| {
            berserk_auth::AuthError::new(
                berserk_auth::ErrorKind::Configuration,
                "system clock is before the Unix epoch",
            )
        })?
        .as_secs();
    match guard.authenticate(token, now) {
        Ok(principal) => Ok(principal),
        Err(error) if error.is_authentication_failure() => Ok(None),
        Err(error) => Err(error.into()),
    }
}

#[cfg(feature = "auth")]
fn bearer_token(value: &str) -> Option<&str> {
    if value.len() > 8192 {
        return None;
    }
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim_start_matches(' ');
    let unpadded = token.trim_end_matches('=');
    if !scheme.eq_ignore_ascii_case("Bearer")
        || unpadded.is_empty()
        || !unpadded
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._~+/".contains(&byte))
    {
        return None;
    }
    Some(token)
}

/// Place inside response-decorating middleware to preserve headers on rendered errors.
pub struct HandleErrors;
impl Middleware for HandleErrors {
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {
        Ok(next.run(request).unwrap_or_else(|error| error.response()))
    }
}
#[cfg(feature = "auth")]
pub struct RequireAbility(pub berserk_auth::Ability);
#[cfg(feature = "auth")]
impl RequireAbility {
    pub fn new(ability: impl Into<String>) -> Result<Self> {
        Ok(Self(berserk_auth::Ability::new(ability)?))
    }
}
#[cfg(feature = "auth")]
impl Middleware for RequireAbility {
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {
        let principal = request.user().ok_or_else(crate::Error::unauthorized)?;
        if let Some(gate) = request.state::<berserk_auth::Gate>() {
            gate.authorize(principal, &self.0)?;
        } else if !principal.can(self.0.as_str()) {
            return Err(crate::Error::forbidden());
        }
        next.run(request)
    }
}

#[cfg(all(test, feature = "auth"))]
mod auth_tests {
    use super::*;
    use crate::{App, Headers, Method};
    use berserk_auth::{
        Ability, AuthError, Decision, ErrorKind, Guard, MemoryTokenStore, Policy, Principal,
        TokenManager,
    };

    struct TestGuard;
    impl Guard for TestGuard {
        fn authenticate(&self, token: &str, _: u64) -> berserk_auth::Result<Option<Principal>> {
            match token {
                "accepted" => Ok(Some(
                    Principal::new("user:1")
                        .unwrap()
                        .with_abilities(["posts.update"])
                        .unwrap(),
                )),
                "read-only" => Ok(Some(
                    Principal::new("user:2")
                        .unwrap()
                        .with_abilities(["posts.read"])
                        .unwrap(),
                )),
                "expired" => Err(AuthError::new(ErrorKind::ExpiredToken, "private detail")),
                "store-error" => Err(AuthError::new(ErrorKind::Store, "private detail")),
                _ => Ok(None),
            }
        }
    }

    fn request(path: &str, values: &[&str]) -> Request {
        let mut headers = Headers::new();
        for value in values {
            headers.append("Authorization", value).unwrap();
        }
        Request::new(Method::new("GET").unwrap(), path, headers, vec![]).unwrap()
    }

    #[test]
    fn configured_auth_keeps_route_definitions_concise() {
        let mut app = App::new();
        app.auth(TestGuard).unwrap();
        assert!(app.auth(TestGuard).is_err());

        app.route()
            .auth()
            .get("/profile", |req: Request| {
                Response::text(req.user().unwrap().subject())
            })
            .unwrap();
        app.route()
            .guest()
            .get("/login", || Response::text("guest"))
            .unwrap();
        app.route()
            .can("posts.update")
            .unwrap()
            .get("/posts/1", || Response::text("allowed"))
            .unwrap();

        assert_eq!(app.respond(request("/profile", &[])).status_code(), 401);
        assert_eq!(
            app.respond(request("/profile", &["Bearer accepted"]))
                .body(),
            b"user:1"
        );
        assert_eq!(app.respond(request("/login", &[])).status_code(), 200);
        assert_eq!(
            app.respond(request("/login", &["Bearer accepted"]))
                .status_code(),
            403
        );
        assert_eq!(
            app.respond(request("/posts/1", &["Bearer accepted"]))
                .status_code(),
            200
        );
        assert_eq!(
            app.respond(request("/posts/1", &["Bearer read-only"]))
                .status_code(),
            403
        );
        assert!(app.route().can("bad ability").is_err());
    }

    #[test]
    fn authentication_rejects_ambiguous_or_malformed_credentials() {
        let mut app = App::new();
        app.route()
            .middleware(Authenticated::new(TestGuard))
            .get("/private", |req: Request| {
                assert_eq!(req.user(), req.principal());
                assert!(req.can("posts.update"));
                Response::text(req.user().unwrap().subject())
            })
            .unwrap();
        for values in [
            vec![],
            vec![""],
            vec!["Bearer"],
            vec!["Bearer "],
            vec!["Basic accepted"],
            vec!["Bearer accepted extra"],
            vec!["Bearer accepted,accepted"],
            vec!["Bearer\taccepted"],
            vec!["Bearer accepted\textra"],
            vec!["Bearer ="],
            vec!["Bearer a=b"],
            vec!["Bearer rejected"],
            vec!["Bearer expired"],
            vec!["Bearer accepted", "Bearer accepted"],
            vec!["Bearer accepted", "Basic ignored"],
        ] {
            let response = app.respond(request("/private", &values));
            assert_eq!(response.status_code(), 401, "{values:?}");
            assert_eq!(response.headers().get("www-authenticate"), Some("Bearer"));
            assert!(!String::from_utf8_lossy(response.body()).contains("private detail"));
        }
        for value in [
            "Bearer accepted",
            "bEaReR accepted",
            "Bearer   accepted",
            "Bearer accepted ",
        ] {
            assert_eq!(app.respond(request("/private", &[value])).body(), b"user:1");
        }
        let failure = app.respond(request("/private", &["Bearer store-error"]));
        assert_eq!(failure.status_code(), 500);
        assert!(!String::from_utf8_lossy(failure.body()).contains("private detail"));
        assert!(bearer_token(&format!("Bearer {}", "a".repeat(8192))).is_none());
    }

    #[test]
    fn guest_routes_allow_absent_or_invalid_credentials_and_reject_authenticated_users() {
        let mut app = App::new();
        app.route()
            .middleware(Guest::new(TestGuard))
            .get("/login", |req: Request| {
                assert!(req.user().is_none());
                assert!(!req.can("posts.update"));
                Response::text("guest")
            })
            .unwrap();
        for values in [vec![], vec!["Bearer rejected"], vec!["Bearer expired"]] {
            assert_eq!(app.respond(request("/login", &values)).status_code(), 200);
        }
        assert_eq!(
            app.respond(request("/login", &["Bearer accepted"]))
                .status_code(),
            403
        );
        for values in [
            vec!["Basic accepted"],
            vec!["Bearer accepted", "Bearer rejected"],
        ] {
            assert_eq!(app.respond(request("/login", &values)).status_code(), 401);
        }
        let response = app.respond(request("/login", &["Bearer store-error"]));
        assert_eq!(response.status_code(), 500);
        assert!(!String::from_utf8_lossy(response.body()).contains("private detail"));
    }

    struct PostPolicy;
    impl Policy<String> for PostPolicy {
        fn authorize(&self, principal: &Principal, _: &Ability, owner: &String) -> Decision {
            if principal.subject() == owner {
                Decision::Allow
            } else {
                Decision::Deny
            }
        }
    }

    #[test]
    fn controller_authorization_requires_identity_scope_and_policy() {
        let guest = request("/", &[]);
        assert_eq!(
            guest
                .authorize(&PostPolicy, "posts.update", &"user:1".to_owned())
                .unwrap_err()
                .status_code(),
            401
        );
        let mut app = App::new();
        for (path, ability, owner) in [
            ("/allowed", "posts.update", "user:1"),
            ("/denied", "posts.update", "user:2"),
            ("/scope", "posts.delete", "user:1"),
        ] {
            app.route()
                .middleware(Authenticated::new(TestGuard))
                .get(path, move |req: Request| -> Result<Response> {
                    req.authorize(&PostPolicy, ability, &owner.to_owned())?;
                    Ok(Response::text("allowed"))
                })
                .unwrap();
        }
        for (path, status) in [("/allowed", 200), ("/denied", 403), ("/scope", 403)] {
            assert_eq!(
                app.respond(request(path, &["Bearer accepted"]))
                    .status_code(),
                status
            );
        }
    }

    #[test]
    fn real_tokens_support_route_abilities_and_revocation() {
        let tokens = Arc::new(TokenManager::new(MemoryTokenStore::default()));
        let token = tokens
            .issue(Principal::new("user:1").unwrap(), ["posts.update"], 0)
            .unwrap();
        let header = format!("Bearer {}", token.expose());
        let mut app = App::new();
        app.route()
            .middleware(Authenticated::new(tokens.clone()))
            .middleware(RequireAbility::new("posts.update").unwrap())
            .get("/private", || Response::text("allowed"))
            .unwrap();
        assert_eq!(
            app.respond(request("/private", &[&header])).status_code(),
            200
        );
        let read_only = tokens
            .issue(Principal::new("user:2").unwrap(), ["posts.read"], 0)
            .unwrap();
        assert_eq!(
            app.respond(request(
                "/private",
                &[&format!("Bearer {}", read_only.expose())]
            ))
            .status_code(),
            403
        );
        tokens.revoke(&token.digest(), 1).unwrap();
        assert_eq!(
            app.respond(request("/private", &[&header])).status_code(),
            401
        );
        app.route()
            .middleware(RequireAbility::new("posts.update").unwrap())
            .get("/guest", || Response::text("unreachable"))
            .unwrap();
        assert_eq!(app.respond(request("/guest", &[])).status_code(), 401);
    }
}
