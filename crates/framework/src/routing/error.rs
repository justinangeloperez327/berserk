use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RouteError {
    InvalidPattern,
    InvalidRouteName,
    InvalidResourcePath,
    DuplicateParameter,
    DuplicateRoute,
    DuplicateRouteName(String),
    DuplicateFallback,
    ScopedFallback,
    UnknownRouteName(String),
    MissingRouteParameter(String),
    UnknownRouteParameter(String),
    ParameterCountMismatch { expected: usize, actual: usize },
}
impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPattern => f.write_str("invalid route pattern"),
            Self::InvalidRouteName => f.write_str("invalid route name"),
            Self::InvalidResourcePath => {
                f.write_str("resource path must be a non-root static path")
            }
            Self::DuplicateParameter => f.write_str("duplicate parameter name in route"),
            Self::DuplicateRoute => f.write_str("equivalent route already registered for method"),
            Self::DuplicateRouteName(name) => {
                write!(f, "route name '{name}' is already registered")
            }
            Self::DuplicateFallback => f.write_str("fallback route already registered"),
            Self::ScopedFallback => {
                f.write_str("fallback routes cannot be registered under a path prefix")
            }
            Self::UnknownRouteName(name) => write!(f, "unknown route name '{name}'"),
            Self::MissingRouteParameter(name) => {
                write!(f, "missing value for route parameter '{name}'")
            }
            Self::UnknownRouteParameter(name) => {
                write!(f, "unknown route parameter '{name}'")
            }
            Self::ParameterCountMismatch { expected, actual } => write!(
                f,
                "controller expects {expected} typed route parameters but route defines {actual}"
            ),
        }
    }
}
impl Error for RouteError {}
