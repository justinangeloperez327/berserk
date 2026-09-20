//! BERSERK application assembly with optional, bounded Tokio/Hyper HTTP serving.
#![forbid(unsafe_code)]

mod app;
mod config;
pub mod controller;
mod error;
pub mod http;
pub mod prelude;
pub mod security;
pub mod support;

pub use app::App;
pub use berserk_core::{ConfigError, ShutdownHandle, State, Validate};
pub use config::ServerConfig;
pub use controller::ActionResult;
pub use error::{Error, Result};
pub use security::{Cors, SecurityHeaders};
pub use support::{Arr, Str};

#[cfg(feature = "auth")]
pub use berserk_auth as auth;
#[cfg(feature = "cache")]
pub use berserk_cache as cache;
#[cfg(feature = "cli")]
pub use berserk_cli as cli;
#[cfg(feature = "client")]
pub use berserk_client as client;
#[cfg(feature = "database")]
pub use berserk_database as database;
#[cfg(feature = "events")]
pub use berserk_events as events;
#[cfg(feature = "jobs")]
pub use berserk_jobs as jobs;
#[cfg(feature = "notifications")]
pub use berserk_notifications as notifications;
#[cfg(feature = "openapi")]
pub use berserk_openapi as openapi;
#[cfg(feature = "storage")]
pub use berserk_storage as storage;
#[cfg(feature = "claw")]
pub use claw_orm as claw;

#[cfg(feature = "database")]
pub use http::RequestConnection;
pub use http::{Headers, HttpError, IntoResponse, Method, Request, Response, StatusCode};

pub mod routing;
pub use routing::{ApiResourceController, NamedRoute, ResourceController, Route, RouteError};

#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "server")]
pub use server::Server;

pub mod middleware;
mod state;
#[cfg(feature = "auth")]
pub use middleware::{Authenticated, Guest};
pub use middleware::{Middleware, Next, RequestId};

pub mod input;
pub mod json;
pub use input::{
    FormRequest, FromJson, ValidateInput, Validated, ValidationErrors, ValidationResult,
};
pub use json::Json;

pub mod multipart;

mod resource;
pub use resource::{ApiResource, Resource, ResourceCollection};

pub mod operational;
pub use operational::{
    HealthCheck, HealthCheckResult, HealthRegistry, HealthSnapshot, HealthStatus, LogEvent,
    LogSink, MemoryLogSink, Metrics,
    MetricsLayer, MetricsSnapshot, RateLimitDecision, RateLimitLayer, RateLimiter, RequestLogger,
    StderrJson, TraceContext, TraceLayer,
};

mod responses;
pub use responses::{redirect, response, ResponseFactory};
#[cfg(feature = "auth")]
mod authorization;
#[cfg(feature = "auth")]
pub use authorization::Auth;
pub use middleware::HandleErrors;
#[cfg(feature = "auth")]
pub use middleware::RequireAbility;

#[cfg(feature = "claw")]
pub use routing::CrudController;
