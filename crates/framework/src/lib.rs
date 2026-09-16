//! Framework assembly. Synchronous HTTP serving with bounded workers.
#![forbid(unsafe_code)]

mod app;
mod config;
pub mod controller;
mod error;
pub mod http;
pub mod prelude;

pub use app::App;
pub use config::ServerConfig;
pub use controller::ActionResult;
pub use error::{Error, Result};
pub use framework_core::{ConfigError, ShutdownHandle, State, Validate};

#[cfg(feature = "claw")]
pub use claw_orm as claw;
#[cfg(feature = "auth")]
pub use framework_auth as auth;
#[cfg(feature = "cache")]
pub use framework_cache as cache;
#[cfg(feature = "cli")]
pub use framework_cli as cli;
#[cfg(feature = "client")]
pub use framework_client as client;
#[cfg(feature = "database")]
pub use framework_database as database;
#[cfg(feature = "events")]
pub use framework_events as events;
#[cfg(feature = "jobs")]
pub use framework_jobs as jobs;
#[cfg(feature = "notifications")]
pub use framework_notifications as notifications;
#[cfg(feature = "openapi")]
pub use framework_openapi as openapi;
#[cfg(feature = "storage")]
pub use framework_storage as storage;

#[cfg(feature = "database")]
pub use http::RequestConnection;
pub use http::{Headers, HttpError, IntoResponse, Method, Request, Response, StatusCode};

pub mod routing;
pub use routing::{ApiResourceController, NamedRoute, ResourceController, Route, RouteError};

pub mod server;
pub use server::Server;

pub mod middleware;
mod state;
#[cfg(feature = "auth")]
pub use middleware::Authenticated;
pub use middleware::{Middleware, Next, RequestId};

pub mod input;
pub mod json;
pub use input::{FromJson, ValidateInput, Validated, ValidationErrors};
pub use json::Json;

pub mod multipart;

mod resource;
pub use resource::{ApiResource, Resource, ResourceCollection};

pub mod operational;
pub use operational::{
    HealthCheck, HealthRegistry, HealthStatus, LogEvent, LogSink, MemoryLogSink, Metrics,
    MetricsLayer, MetricsSnapshot, RateLimitDecision, RateLimitLayer, RateLimiter, RequestLogger,
    StderrJson, TraceContext, TraceLayer,
};
