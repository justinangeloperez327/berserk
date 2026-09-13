use std::{error::Error, fmt};

pub type Result<T> = std::result::Result<T, CacheError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheError(String);
impl CacheError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
impl fmt::Display for CacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl Error for CacheError {}
