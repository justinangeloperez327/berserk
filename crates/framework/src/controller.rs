//! Controller-oriented handler adapters.
use crate::{FromJson, IntoResponse, Request, Response, Result, ValidateInput, Validated};
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
pub struct ValidatedArg<T>(PhantomData<fn() -> T>);

#[doc(hidden)]
pub struct ValidatedRequest<T>(PhantomData<fn() -> T>);

#[doc(hidden)]
pub struct RouteParamValidated<T, I>(PhantomData<fn() -> (T, I)>);

#[doc(hidden)]
pub struct RouteParamValidatedRequest<T, I>(PhantomData<fn() -> (T, I)>);

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

impl<F, T, R> Handler<ValidatedArg<T>> for F
where
    F: Fn(Validated<T>) -> R + Send + Sync + 'static,
    T: FromJson + ValidateInput + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(0)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let value = request.validated::<T>()?;
        self(Validated::new(value)).into_response()
    }
}

impl<F, T, R> Handler<ValidatedRequest<T>> for F
where
    F: Fn(Validated<T>, Request) -> R + Send + Sync + 'static,
    T: FromJson + ValidateInput + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(0)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let value = request.validated::<T>()?;
        self(Validated::new(value), request).into_response()
    }
}

impl<F, T, I, R> Handler<RouteParamValidated<T, I>> for F
where
    F: Fn(T, Validated<I>) -> R + Send + Sync + 'static,
    T: FromStr + 'static,
    I: FromJson + ValidateInput + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(1)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let route_param = match request.single_param_as::<T>() {
            Some(Ok(value)) => value,
            Some(Err(_)) | None => {
                return Ok(Response::text("Invalid route parameter").status(400));
            }
        };
        let input = request.validated::<I>()?;
        self(route_param, Validated::new(input)).into_response()
    }
}

impl<F, T, I, R> Handler<RouteParamValidatedRequest<T, I>> for F
where
    F: Fn(T, Validated<I>, Request) -> R + Send + Sync + 'static,
    T: FromStr + 'static,
    I: FromJson + ValidateInput + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(1)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let route_param = match request.single_param_as::<T>() {
            Some(Ok(value)) => value,
            Some(Err(_)) | None => {
                return Ok(Response::text("Invalid route parameter").status(400));
            }
        };
        let input = request.validated::<I>()?;
        self(route_param, Validated::new(input), request).into_response()
    }
}
