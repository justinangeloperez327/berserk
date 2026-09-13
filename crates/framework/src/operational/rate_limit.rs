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
