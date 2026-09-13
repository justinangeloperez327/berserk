use std::{error::Error, fmt};

pub type Result<T> = std::result::Result<T, OpenApiError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenApiError(String);

impl OpenApiError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
impl fmt::Display for OpenApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl Error for OpenApiError {}
