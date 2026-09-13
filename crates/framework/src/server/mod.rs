//! Synchronous HTTP server and codecs.
mod error;
mod parser;
mod writer;
pub use error::ProtocolError;
pub use parser::read_request;
pub use writer::{write_response, write_response_connection};

mod connection;
mod listener;
pub use listener::Server;

mod stats;
pub use stats::{ServerSnapshot, ServerStats};
