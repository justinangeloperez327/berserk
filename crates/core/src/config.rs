use crate::ConfigError;

/// Configuration must be checked before resources are started.
pub trait Validate {
    fn validate(&self) -> Result<(), ConfigError>;
}
