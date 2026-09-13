use std::{error::Error, fmt, str::Utf8Error};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HttpError {
    InvalidMethod,
    InvalidTarget,
    InvalidHeaderName,
    InvalidHeaderValue,
    ReservedResponseHeader,
    InvalidStatus(u16),
    BodyNotAllowed(u16),
    InvalidUtf8(Utf8Error),
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMethod => f.write_str("invalid HTTP method token"),
            Self::InvalidTarget => f.write_str("invalid origin-form request target"),
            Self::InvalidHeaderName => f.write_str("invalid HTTP header name"),
            Self::InvalidHeaderValue => f.write_str("unsupported HTTP header value"),
            Self::ReservedResponseHeader => {
                f.write_str("response framing headers are encoder-owned")
            }
            Self::InvalidStatus(code) => write!(f, "unsupported final response status: {code}"),
            Self::BodyNotAllowed(code) => {
                write!(f, "response status {code} does not permit a body")
            }
            Self::InvalidUtf8(_) => f.write_str("request body is not valid UTF-8"),
        }
    }
}
impl Error for HttpError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidUtf8(error) => Some(error),
            _ => None,
        }
    }
}
