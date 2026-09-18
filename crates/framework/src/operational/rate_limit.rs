use super::{Metrics, OperationalError};
use crate::{Middleware, Next, Request, Response, Result};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RateLimitDecision {
    pub allowed: bool,
    pub limit: u64,
    pub remaining: u64,
    pub reset_at: u64,
}

#[derive(Clone, Copy, Debug)]
struct Window {
    started_at: u64,
    used: u64,
}

pub struct RateLimiter {
    limit: u64,
    window_secs: u64,
    max_keys: usize,
    windows: Mutex<HashMap<String, Window>>,
}

impl RateLimiter {
    pub fn new(
        limit: u64,
        window: Duration,
        max_keys: usize,
    ) -> std::result::Result<Self, OperationalError> {
        if limit == 0 {
            return Err(OperationalError::new(
                "rate limit must be greater than zero",
            ));
        }
        if window.as_secs() == 0 {
            return Err(OperationalError::new(
                "rate-limit window must be at least one second",
            ));
        }
        if max_keys == 0 {
            return Err(OperationalError::new(
                "rate-limit key capacity must be greater than zero",
            ));
        }
        Ok(Self {
            limit,
            window_secs: window.as_secs(),
            max_keys,
            windows: Mutex::new(HashMap::new()),
        })
    }
    pub fn check(
        &self,
        key: &str,
        now: u64,
    ) -> std::result::Result<RateLimitDecision, OperationalError> {
        if key.is_empty() || key.len() > 128 {
            return Err(OperationalError::new(
                "rate-limit keys must contain 1 to 128 bytes",
            ));
        }
        let mut windows = self
            .windows
            .lock()
            .map_err(|_| OperationalError::new("rate limiter lock poisoned"))?;
        windows.retain(|_, window| now < window.started_at.saturating_add(self.window_secs));
        if !windows.contains_key(key) && windows.len() >= self.max_keys {
            return Ok(RateLimitDecision {
                allowed: false,
                limit: self.limit,
                remaining: 0,
                reset_at: now.saturating_add(self.window_secs),
            });
        }
        let window = windows.entry(key.to_owned()).or_insert(Window {
            started_at: now,
            used: 0,
        });
        if now < window.started_at || now >= window.started_at.saturating_add(self.window_secs) {
            *window = Window {
                started_at: now,
                used: 0,
            };
        }
        let allowed = window.used < self.limit;
        if allowed {
            window.used += 1;
        }
        Ok(RateLimitDecision {
            allowed,
            limit: self.limit,
            remaining: self.limit.saturating_sub(window.used),
            reset_at: window.started_at.saturating_add(self.window_secs),
        })
    }
}

type Keyer = Arc<dyn Fn(&Request) -> String + Send + Sync>;

pub struct RateLimitLayer {
    limiter: Arc<RateLimiter>,
    keyer: Keyer,
    metrics: Option<Arc<Metrics>>,
}
impl RateLimitLayer {
    /// Put this after authentication middleware. Guests share one bucket.
    /// Subjects are hashed to fit the key bound without exposing identity in keys.
    #[cfg(feature = "auth")]
    pub fn per_user(limiter: Arc<RateLimiter>) -> Self {
        use std::{collections::hash_map::RandomState, hash::BuildHasher, sync::OnceLock};
        static USER_KEYS: OnceLock<RandomState> = OnceLock::new();
        let hasher = USER_KEYS.get_or_init(RandomState::new);
        Self::keyed(limiter, move |request| {
            request.user().map_or_else(
                || "guest".to_owned(),
                |user| format!("user:{:016x}", hasher.hash_one(user.subject())),
            )
        })
    }

    pub fn global(limiter: Arc<RateLimiter>) -> Self {
        Self {
            limiter,
            keyer: Arc::new(|_| "global".into()),
            metrics: None,
        }
    }
    pub fn keyed(
        limiter: Arc<RateLimiter>,
        keyer: impl Fn(&Request) -> String + Send + Sync + 'static,
    ) -> Self {
        Self {
            limiter,
            keyer: Arc::new(keyer),
            metrics: None,
        }
    }
    pub fn metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }
}

impl Middleware for RateLimitLayer {
    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| OperationalError::new("system clock is before Unix epoch"))?
            .as_secs();
        let decision = self.limiter.check(&(self.keyer)(&request), now)?;
        if !decision.allowed {
            if let Some(metrics) = &self.metrics {
                metrics.mark_rate_limited();
            }
            let retry = decision.reset_at.saturating_sub(now).max(1).to_string();
            return Response::text("Too Many Requests")
                .status(429)
                .header("retry-after", &retry)?
                .header("x-ratelimit-limit", &decision.limit.to_string())?
                .header("x-ratelimit-remaining", "0");
        }
        next.run(request)?
            .header("x-ratelimit-limit", &decision.limit.to_string())?
            .header("x-ratelimit-remaining", &decision.remaining.to_string())?
            .header("x-ratelimit-reset", &decision.reset_at.to_string())
    }
}

#[cfg(all(test, feature = "auth"))]
mod auth_tests {
    use super::*;
    use crate::{App, Authenticated, Headers, Method};
    use berserk_auth::{Guard, Principal};

    struct Users;
    impl Guard for Users {
        fn authenticate(&self, token: &str, _: u64) -> berserk_auth::Result<Option<Principal>> {
            // Two different credentials deliberately resolve to the same user.
            Ok(Principal::new(if token == "one" || token == "one-again" { "user:1" } else { "user:2" }))
        }
    }

    fn request(token: &str) -> Request {
        let mut headers = Headers::new();
        headers.insert("authorization", &format!("Bearer {token}")).unwrap();
        Request::new(Method::new("GET").unwrap(), "/", headers, vec![]).unwrap()
    }

    #[test]
    fn per_user_buckets_follow_authenticated_identity() {
        let limiter = Arc::new(RateLimiter::new(1, Duration::from_secs(3600), 10).unwrap());
        let mut app = App::new();
        app.route().middleware(Authenticated::new(Users))
            .middleware(RateLimitLayer::per_user(limiter))
            .get("/", || Response::text("ok")).unwrap();
        assert_eq!(app.respond(request("one")).status_code(), 200);
        let limited = app.respond(request("one-again"));
        assert_eq!(limited.status_code(), 429);
        assert!(limited.headers().get("retry-after").is_some());
        assert_eq!(app.respond(request("two")).status_code(), 200);
    }

    #[test]
    fn guest_and_long_subject_keys_are_bounded_and_distinct() {
        let limiter = Arc::new(RateLimiter::new(1, Duration::from_secs(3600), 10).unwrap());
        let layer = RateLimitLayer::per_user(limiter.clone());
        let mut request = request("one");
        let guest_key = (layer.keyer)(&request);
        request.set_principal(Principal::new("guest").unwrap());
        assert_ne!(guest_key, (layer.keyer)(&request));
        request.set_principal(Principal::new("u".repeat(1024)).unwrap());
        let key = (layer.keyer)(&request);
        let another_layer = RateLimitLayer::per_user(limiter.clone());
        assert_eq!(key, (another_layer.keyer)(&request));
        assert!(key.len() <= 128);
        assert!(limiter.check(&key, 0).unwrap().allowed);
        assert!(!limiter.check(&key, 0).unwrap().allowed);
        assert!(limiter.check(&guest_key, 0).unwrap().allowed);
    }
}
