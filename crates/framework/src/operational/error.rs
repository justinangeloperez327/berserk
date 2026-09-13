use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationalError(String);

impl OperationalError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
impl fmt::Display for OperationalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl Error for OperationalError {}
