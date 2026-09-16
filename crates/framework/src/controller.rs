//! Controller-oriented handler adapters.
use crate::{IntoResponse, Request, Response, Result};
use std::{marker::PhantomData, str::FromStr};

/// Conventional result type for controller actions that can fail.
pub type ActionResult = Result<Response>;

#[doc(hidden)]
pub struct NoArgs;

#[doc(hidden)]
pub struct RequestArg;

#[doc(hidden)]
pub struct RouteParam<T>(PhantomData<fn() -> T>);

#[doc(hidden)]
pub struct RouteParamRequest<T>(PhantomData<fn() -> T>);

#[doc(hidden)]
pub trait Handler<Args>: Send + Sync + 'static {
    fn expected_route_params() -> Option<usize>;
    fn call(&self, request: Request) -> Result<Response>;
}

impl<F, R> Handler<NoArgs> for F
where
    F: Fn() -> R + Send + Sync + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(0)
    }

    fn call(&self, _request: Request) -> Result<Response> {
        self().into_response()
    }
}

impl<F, R> Handler<RequestArg> for F
where
    F: Fn(Request) -> R + Send + Sync + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        None
    }

    fn call(&self, request: Request) -> Result<Response> {
        self(request).into_response()
    }
}

impl<F, T, R> Handler<RouteParam<T>> for F
where
    F: Fn(T) -> R + Send + Sync + 'static,
    T: FromStr + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(1)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let value = match request.single_param_as::<T>() {
            Some(Ok(value)) => value,
            Some(Err(_)) | None => {
                return Ok(Response::text("Invalid route parameter").status(400));
            }
        };
        self(value).into_response()
    }
}

impl<F, T, R> Handler<RouteParamRequest<T>> for F
where
    F: Fn(T, Request) -> R + Send + Sync + 'static,
    T: FromStr + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(1)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let value = match request.single_param_as::<T>() {
            Some(Ok(value)) => value,
            Some(Err(_)) | None => {
                return Ok(Response::text("Invalid route parameter").status(400));
            }
        };
        self(value, request).into_response()
    }
}
