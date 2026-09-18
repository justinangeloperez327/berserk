use std::{error::Error, fmt};

pub type Result<T> = std::result::Result<T, AuthError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Configuration,
    Crypto,
    InvalidCredentials,
    InvalidToken,
    ExpiredToken,
    RevokedToken,
    Unauthorized,
    Forbidden,
    Store,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthError {
    kind: ErrorKind,
    message: String,
}

impl AuthError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Credential failures should share one public 401 response.
    /// Store, configuration, and cryptographic failures remain server errors.
    pub const fn is_authentication_failure(&self) -> bool {
        matches!(
            self.kind,
            ErrorKind::InvalidCredentials
                | ErrorKind::InvalidToken
                | ErrorKind::ExpiredToken
                | ErrorKind::RevokedToken
                | ErrorKind::Unauthorized
        )
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl Error for AuthError {}
