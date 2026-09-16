//! Controller-oriented handler adapters.
use crate::{IntoResponse, Request, Response, Result};
use std::str::FromStr;

/// Conventional result type for controller actions that can fail.
pub type ActionResult = Result<Response>;

pub(crate) fn with_param<T, F, R>(
    parameter: String,
    handler: F,
) -> impl Fn(Request) -> Result<Response> + Send + Sync + 'static
where
    T: FromStr + 'static,
    F: Fn(T) -> R + Send + Sync + 'static,
    R: IntoResponse,
{
    move |request| {
        let value = match request.param_as::<T>(&parameter) {
            Some(Ok(value)) => value,
            Some(Err(_)) | None => {
                return Ok(Response::text("Invalid route parameter").status(400));
            }
        };
        handler(value).into_response()
    }
}
