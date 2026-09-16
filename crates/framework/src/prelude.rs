//! Common framework imports.
#[cfg(feature = "auth")]
pub use crate::Authenticated;
pub use crate::{
    ActionResult, ApiResource, App, Error, IntoResponse, Request, Resource, ResourceCollection,
    Response, Result, ServerConfig, State,
};
pub use crate::{Metrics, MetricsLayer, RateLimitLayer, RateLimiter, RequestLogger, TraceLayer};
