//! Small, transport-independent foundations. No external dependencies.
#![forbid(unsafe_code)]

mod config;
mod error;
mod lifecycle;
mod state;

pub use config::Validate;
pub use error::ConfigError;
pub use lifecycle::ShutdownHandle;
pub use state::State;
