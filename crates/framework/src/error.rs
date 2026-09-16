use framework_core::ConfigError;
use std::{error::Error as StdError, fmt};

/// Errors currently implemented by the framework.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    Configuration(ConfigError),
    Http(crate::HttpError),
    Input(crate::input::InputError),
    Server(std::io::Error),
    Routing(crate::RouteError),
    #[cfg(feature = "database")]
    Database(framework_database::DatabaseError),
    #[cfg(feature = "auth")]
    Auth(framework_auth::AuthError),
    #[cfg(feature = "openapi")]
    OpenApi(framework_openapi::OpenApiError),
    Operational(crate::operational::OperationalError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<ConfigError> for Error {
    fn from(error: ConfigError) -> Self {
        Self::Configuration(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Server(error) => fmt::Display::fmt(error, f),
            Self::Input(error) => fmt::Display::fmt(error, f),
            Self::Configuration(error) => fmt::Display::fmt(error, f),
            Self::Http(error) => fmt::Display::fmt(error, f),
            Self::Routing(error) => fmt::Display::fmt(error, f),
            #[cfg(feature = "database")]
            Self::Database(error) => fmt::Display::fmt(error, f),
            Self::Operational(error) => fmt::Display::fmt(error, f),
            #[cfg(feature = "auth")]
            Self::Auth(error) => fmt::Display::fmt(error, f),
            #[cfg(feature = "openapi")]
            Self::OpenApi(error) => fmt::Display::fmt(error, f),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Server(error) => Some(error),
            Self::Input(error) => Some(error),
            Self::Configuration(error) => Some(error),
            Self::Http(error) => Some(error),
            Self::Routing(error) => Some(error),
            #[cfg(feature = "database")]
            Self::Database(error) => Some(error),
            Self::Operational(error) => Some(error),
            #[cfg(feature = "auth")]
            Self::Auth(error) => Some(error),
            #[cfg(feature = "openapi")]
            Self::OpenApi(error) => Some(error),
        }
    }
}

impl From<crate::HttpError> for Error {
    fn from(error: crate::HttpError) -> Self {
        Self::Http(error)
    }
}

impl From<crate::RouteError> for Error {
    fn from(error: crate::RouteError) -> Self {
        Self::Routing(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Server(error)
    }
}

impl From<crate::input::InputError> for Error {
    fn from(e: crate::input::InputError) -> Self {
        Self::Input(e)
    }
}

#[cfg(feature = "database")]
impl From<framework_database::DatabaseError> for Error {
    fn from(error: framework_database::DatabaseError) -> Self {
        Self::Database(error)
    }
}

#[cfg(feature = "auth")]
impl From<framework_auth::AuthError> for Error {
    fn from(error: framework_auth::AuthError) -> Self {
        Self::Auth(error)
    }
}

#[cfg(feature = "openapi")]
impl From<framework_openapi::OpenApiError> for Error {
    fn from(error: framework_openapi::OpenApiError) -> Self {
        Self::OpenApi(error)
    }
}

impl From<crate::operational::OperationalError> for Error {
    fn from(error: crate::operational::OperationalError) -> Self {
        Self::Operational(error)
    }
}
