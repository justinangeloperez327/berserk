use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RouteError {
    InvalidPattern,
    DuplicateParameter,
    DuplicateRoute,
    DuplicateFallback,
    ScopedFallback,
    ParameterCountMismatch { expected: usize, actual: usize },
}
impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPattern => f.write_str("invalid route pattern"),
            Self::DuplicateParameter => f.write_str("duplicate parameter name in route"),
            Self::DuplicateRoute => f.write_str("equivalent route already registered for method"),
            Self::DuplicateFallback => f.write_str("fallback route already registered"),
            Self::ScopedFallback => {
                f.write_str("fallback routes cannot be registered under a path prefix")
            }
            Self::ParameterCountMismatch { expected, actual } => write!(
                f,
                "controller expects {expected} typed route parameters but route defines {actual}"
            ),
        }
    }
}
impl Error for RouteError {}
