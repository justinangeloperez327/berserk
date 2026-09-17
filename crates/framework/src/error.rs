use berserk_core::ConfigError;
use std::{error::Error as StdError, fmt};

/// Errors currently implemented by the framework.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    Rejected {
        status: u16,
        message: String,
    },
    Configuration(ConfigError),
    Http(crate::HttpError),
    Input(crate::input::InputError),
    Server(std::io::Error),
    Routing(crate::RouteError),
    #[cfg(feature = "database")]
    Database(berserk_database::DatabaseError),
    #[cfg(feature = "auth")]
    Auth(berserk_auth::AuthError),
    #[cfg(feature = "openapi")]
    OpenApi(berserk_openapi::OpenApiError),
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
            Self::Rejected { message, .. } => f.write_str(message),
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
            Self::Rejected { .. } => None,
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
impl From<berserk_database::DatabaseError> for Error {
    fn from(error: berserk_database::DatabaseError) -> Self {
        Self::Database(error)
    }
}

#[cfg(feature = "auth")]
impl From<berserk_auth::AuthError> for Error {
    fn from(error: berserk_auth::AuthError) -> Self {
        Self::Auth(error)
    }
}

#[cfg(feature = "openapi")]
impl From<berserk_openapi::OpenApiError> for Error {
    fn from(error: berserk_openapi::OpenApiError) -> Self {
        Self::OpenApi(error)
    }
}

impl From<crate::operational::OperationalError> for Error {
    fn from(error: crate::operational::OperationalError) -> Self {
        Self::Operational(error)
    }
}

impl Error {
    pub fn rejected(status: u16, message: impl Into<String>) -> Self {
        Self::Rejected {
            status: if (400..600).contains(&status) {
                status
            } else {
                500
            },
            message: message.into(),
        }
    }
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::rejected(400, message)
    }
    pub fn unauthorized() -> Self {
        Self::rejected(401, "Unauthorized")
    }
    pub fn forbidden() -> Self {
        Self::rejected(403, "Forbidden")
    }
    pub fn not_found() -> Self {
        Self::rejected(404, "Not Found")
    }
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::rejected(409, message)
    }
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Rejected { status, .. } => *status,
            Self::Input(input) => input.response().status_code(),
            Self::Http(crate::HttpError::InvalidUtf8(_)) => 400,
            #[cfg(feature = "database")]
            Self::Database(error) => match error.kind() {
                berserk_database::ErrorKind::NotFound => 404,
                berserk_database::ErrorKind::InvalidInput => 400,
                berserk_database::ErrorKind::Constraint => 409,
                berserk_database::ErrorKind::Timeout => 503,
                _ => 500,
            },
            #[cfg(feature = "auth")]
            Self::Auth(error) => match error.kind() {
                berserk_auth::ErrorKind::Forbidden => 403,
                berserk_auth::ErrorKind::Unauthorized
                | berserk_auth::ErrorKind::InvalidCredentials => 401,
                _ => 500,
            },
            _ => 500,
        }
    }
    /// Render public errors without exposing database, configuration, or internal details.
    pub fn response(&self) -> crate::Response {
        if let Self::Input(input) = self {
            return input.response();
        }
        let status = self.status_code();
        let message = if status >= 500 {
            "Internal Server Error"
        } else {
            match self {
                Self::Rejected { message, .. } => message,
                _ => match status {
                    400 => "Bad Request",
                    401 => "Unauthorized",
                    403 => "Forbidden",
                    404 => "Not Found",
                    409 => "Conflict",
                    _ => "Request failed",
                },
            }
        };
        let json = crate::Json::Object(
            [
                ("message".into(), crate::Json::from(message)),
                ("errors".into(), crate::Json::Array(vec![])),
            ]
            .into(),
        );
        let response = crate::Response::json(&json)
            .expect("bounded error JSON")
            .status(status);
        if status == 401 {
            response
                .header("www-authenticate", "Bearer")
                .expect("static header")
        } else {
            response
        }
    }
}
