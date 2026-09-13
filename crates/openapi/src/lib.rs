//! Explicit OpenAPI 3.1 document construction.
#![forbid(unsafe_code)]

mod document;
mod error;
mod operation;
mod schema;

pub use document::{Info, OpenApi};
pub use error::{OpenApiError, Result};
pub use operation::{
    ApiResponse, HttpMethod, Operation, Parameter, ParameterLocation, RequestBody, SecurityScheme,
};
pub use schema::{Schema, SchemaRef, SchemaType};
