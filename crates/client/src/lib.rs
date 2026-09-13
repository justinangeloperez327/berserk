//! Bounded synchronous outbound HTTP contracts and a plaintext HTTP transport.
#![forbid(unsafe_code)]

mod error;
mod tcp;
mod types;

pub use error::{ClientError, ErrorKind, Result};
pub use tcp::{TcpClientConfig, TcpHttpClient};
pub use types::{Header, HttpClient, Method, Request, Response, Url};
