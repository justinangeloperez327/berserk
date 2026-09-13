use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RouteError {
    InvalidPattern,
    DuplicateParameter,
    DuplicateRoute,
}
impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidPattern => "invalid route pattern",
            Self::DuplicateParameter => "duplicate parameter name in route",
            Self::DuplicateRoute => "equivalent route already registered for method",
        })
    }
}
impl Error for RouteError {}
