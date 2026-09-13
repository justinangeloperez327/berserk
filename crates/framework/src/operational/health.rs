use super::OperationalError;
use crate::{Json, Response, Result};
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
}

pub trait HealthCheck: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn check(&self) -> HealthStatus;
}
impl<T: HealthCheck + ?Sized> HealthCheck for Arc<T> {
    fn name(&self) -> &'static str {
        (**self).name()
    }
    fn check(&self) -> HealthStatus {
        (**self).check()
    }
}

#[derive(Default)]
pub struct HealthRegistry {
    checks: Vec<Arc<dyn HealthCheck>>,
}

impl HealthRegistry {
    pub fn add(&mut self, check: impl HealthCheck) -> std::result::Result<(), OperationalError> {
        if self.checks.len() >= 64 {
            return Err(OperationalError::new("health check limit is 64"));
        }
        let name = check.name();
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            return Err(OperationalError::new(
                "health check names must be nonempty ASCII identifiers",
            ));
        }
        if self.checks.iter().any(|existing| existing.name() == name) {
            return Err(OperationalError::new(format!(
                "duplicate health check `{name}`"
            )));
        }
        self.checks.push(Arc::new(check));
        Ok(())
    }
    pub fn liveness(&self) -> Result<Response> {
        Ok(Response::text("OK"))
    }
    pub fn readiness(&self) -> Result<Response> {
        let mut healthy = true;
        let checks = self
            .checks
            .iter()
            .map(|check| {
                let status = check.check();
                if status == HealthStatus::Unhealthy {
                    healthy = false;
                }
                Json::Object(BTreeMap::from([
                    ("name".into(), Json::from(check.name())),
                    (
                        "status".into(),
                        Json::from(if status == HealthStatus::Healthy {
                            "healthy"
                        } else {
                            "unhealthy"
                        }),
                    ),
                ]))
            })
            .collect();
        let body = Json::Object(BTreeMap::from([
            (
                "status".into(),
                Json::from(if healthy { "ready" } else { "not_ready" }),
            ),
            ("checks".into(), Json::Array(checks)),
        ]));
        Ok(Response::json(&body)?.status(if healthy { 200 } else { 503 }))
    }
    pub fn names(&self) -> HashSet<&'static str> {
        self.checks.iter().map(|check| check.name()).collect()
    }
}
