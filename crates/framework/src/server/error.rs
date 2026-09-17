use std::{error::Error, fmt, io};

#[derive(Debug)]
#[non_exhaustive]
pub enum ProtocolError {
    Io(io::Error),
    Malformed,
    HeaderLimit,
    BodyLimit,
    UnsupportedVersion,
    UnsupportedTransferEncoding,
    UnsupportedExpectation,
    InvalidConfiguration(crate::ConfigError),
    InvalidResponse(crate::HttpError),
}

impl ProtocolError {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::HeaderLimit => 431,
            Self::BodyLimit => 413,
            Self::UnsupportedVersion => 505,
            Self::UnsupportedTransferEncoding => 501,
            Self::UnsupportedExpectation => 417,
            Self::InvalidConfiguration(_) | Self::InvalidResponse(_) => 500,
            Self::Io(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) =>
            {
                408
            }
            Self::Io(_) | Self::Malformed => 400,
        }
    }
}

pub(super) fn application_error_response(error: crate::Error) -> crate::Response {
    error.response()
}

impl From<io::Error> for ProtocolError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Io(_) => "HTTP transport I/O failed",
            Self::Malformed => "malformed HTTP request",
            Self::HeaderLimit => "request header limit exceeded",
            Self::BodyLimit => "request body limit exceeded",
            Self::UnsupportedVersion => "only HTTP/1.1 is supported",
            Self::UnsupportedTransferEncoding => "transfer encoding is unsupported",
            Self::UnsupportedExpectation => "expectations are unsupported",
            Self::InvalidConfiguration(_) => "invalid codec configuration",
            Self::InvalidResponse(_) => "invalid response before encoding",
        })
    }
}

impl Error for ProtocolError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::InvalidConfiguration(error) => Some(error),
            Self::InvalidResponse(error) => Some(error),
            _ => None,
        }
    }
}
