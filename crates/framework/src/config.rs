use berserk_core::{ConfigError, Validate};
use std::time::Duration;

/// Proposed defaults. Resource enforcement belongs to the server phase.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub workers: usize,
    pub keep_alive: bool,
    pub max_requests_per_connection: usize,
    pub queue_capacity: usize,
    pub max_header_bytes: usize,
    pub max_headers: usize,
    pub max_body_bytes: usize,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub request_deadline: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            workers: 4,
            keep_alive: false,
            max_requests_per_connection: 100,
            queue_capacity: 128,
            max_header_bytes: 16 * 1024,
            max_headers: 100,
            max_body_bytes: 1024 * 1024,
            read_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(5),
            request_deadline: Duration::from_secs(15),
        }
    }
}

impl Validate for ServerConfig {
    fn validate(&self) -> std::result::Result<(), ConfigError> {
        for (field, value) in [
            ("workers", self.workers),
            (
                "max_requests_per_connection",
                self.max_requests_per_connection,
            ),
            ("queue_capacity", self.queue_capacity),
            ("max_header_bytes", self.max_header_bytes),
            ("max_headers", self.max_headers),
        ] {
            if value == 0 {
                return Err(ConfigError::new(field, "must be greater than zero"));
            }
        }
        // Zero body bytes is valid: reject any nonempty request body.
        for (field, value) in [
            ("read_timeout", self.read_timeout),
            ("write_timeout", self.write_timeout),
            ("request_deadline", self.request_deadline),
        ] {
            if value.is_zero() {
                return Err(ConfigError::new(field, "must be greater than zero"));
            }
        }
        if self
            .max_header_bytes
            .checked_add(self.max_body_bytes)
            .is_none()
        {
            return Err(ConfigError::new(
                "max_body_bytes",
                "combined request size overflows usize",
            ));
        }
        Ok(())
    }
}
