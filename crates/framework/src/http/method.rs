use super::{headers::token, HttpError};
use std::{fmt, str::FromStr};

/// Validated, case-sensitive method token, including extension methods.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Method(String);

impl Method {
    pub fn new(value: &str) -> Result<Self, HttpError> {
        if !token(value) {
            return Err(HttpError::InvalidMethod);
        }
        Ok(Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl FromStr for Method {
    type Err = HttpError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}
impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
