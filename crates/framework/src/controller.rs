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
pub struct RouteParams<T, U>(PhantomData<fn() -> (T, U)>);

#[doc(hidden)]
pub struct RouteParamsRequest<T, U>(PhantomData<fn() -> (T, U)>);

#[doc(hidden)]
pub struct RouteParamsValidated<T, U, I>(PhantomData<T>, PhantomData<U>, PhantomData<I>);

#[doc(hidden)]
pub struct RouteParamsValidatedRequest<T, U, I>(PhantomData<T>, PhantomData<U>, PhantomData<I>);

#[doc(hidden)]
pub struct ValidatedArg<T>(PhantomData<fn() -> T>);

#[doc(hidden)]
pub struct ValidatedRequest<T>(PhantomData<fn() -> T>);

#[doc(hidden)]
pub struct RouteParamValidated<T, I>(PhantomData<fn() -> (T, I)>);

#[doc(hidden)]
pub struct RouteParamValidatedRequest<T, I>(PhantomData<fn() -> (T, I)>);

#[cfg(feature = "claw")]
#[doc(hidden)]
pub struct RouteModel<M>(PhantomData<M>);

#[cfg(feature = "claw")]
#[doc(hidden)]
pub struct RouteModelRequest<M>(PhantomData<M>);

#[doc(hidden)]
pub trait Handler<Args>: Send + Sync + 'static {
    fn expected_route_params() -> Option<usize>;
    fn call(&self, request: Request) -> Result<Response>;
}

fn route_param<T: FromStr>(request: &Request, index: usize) -> std::result::Result<T, Response> {
    match request.param_at_as::<T>(index) {
        Some(Ok(value)) => Ok(value),
        Some(Err(_)) | None => Err(Response::text("Invalid route parameter").status(400)),
    }
}

#[cfg(feature = "claw")]
fn route_model<M: claw_orm::Model>(
    request: &Request,
    index: usize,
) -> Result<std::result::Result<M, Response>> {
    let raw = match request.param_at(index) {
        Some(value) => value,
        None => return Ok(Err(Response::text("Invalid route parameter").status(400))),
    };
    let key = match M::parse_route_key(raw) {
        Some(key) => key,
        None => return Ok(Err(Response::text("Invalid route parameter").status(400))),
    };
    let database = request
        .state::<framework_database::Database>()
        .ok_or_else(|| {
            framework_core::ConfigError::new("database", "database state is not configured")
        })?;
    let mut connection = database.acquire()?;
    match M::find(&mut *connection, key)? {
        Some(model) => Ok(Ok(model)),
        None => Ok(Err(Response::text("Not Found").status(404))),
    }
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
        let value = match route_param(&request, 0) {
            Ok(value) => value,
            Err(response) => return Ok(response),
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
        let value = match route_param(&request, 0) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
        self(value, request).into_response()
    }
}

#[cfg(feature = "claw")]
impl<F, M, R> Handler<RouteModel<M>> for F
where
    F: Fn(M) -> R + Send + Sync + 'static,
    M: claw_orm::Model + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(1)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let model = match route_model::<M>(&request, 0)? {
            Ok(model) => model,
            Err(response) => return Ok(response),
        };
        self(model).into_response()
    }
}

#[cfg(feature = "claw")]
impl<F, M, R> Handler<RouteModelRequest<M>> for F
where
    F: Fn(M, Request) -> R + Send + Sync + 'static,
    M: claw_orm::Model + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(1)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let model = match route_model::<M>(&request, 0)? {
            Ok(model) => model,
            Err(response) => return Ok(response),
        };
        self(model, request).into_response()
    }
}

impl<F, T, U, R> Handler<RouteParams<T, U>> for F
where
    F: Fn(T, U) -> R + Send + Sync + 'static,
    T: FromStr + 'static,
    U: FromStr + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(2)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let first = match route_param(&request, 0) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
        let second = match route_param(&request, 1) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
        self(first, second).into_response()
    }
}

impl<F, T, U, R> Handler<RouteParamsRequest<T, U>> for F
where
    F: Fn(T, U, Request) -> R + Send + Sync + 'static,
    T: FromStr + 'static,
    U: FromStr + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(2)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let first = match route_param(&request, 0) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
        let second = match route_param(&request, 1) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
        self(first, second, request).into_response()
    }
}

impl<F, T, U, I, R> Handler<RouteParamsValidated<T, U, I>> for F
where
    F: Fn(T, U, Validated<I>) -> R + Send + Sync + 'static,
    T: FromStr + 'static,
    U: FromStr + 'static,
    I: FromJson + ValidateInput + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(2)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let first = match route_param(&request, 0) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
        let second = match route_param(&request, 1) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
        let input = request.validated::<I>()?;
        self(first, second, Validated::new(input)).into_response()
    }
}

impl<F, T, U, I, R> Handler<RouteParamsValidatedRequest<T, U, I>> for F
where
    F: Fn(T, U, Validated<I>, Request) -> R + Send + Sync + 'static,
    T: FromStr + 'static,
    U: FromStr + 'static,
    I: FromJson + ValidateInput + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(2)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let first = match route_param(&request, 0) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
        let second = match route_param(&request, 1) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
        let input = request.validated::<I>()?;
        self(first, second, Validated::new(input), request)
            .into_response()
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
        let route_param = match route_param(&request, 0) {
            Ok(value) => value,
            Err(response) => return Ok(response),
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
        let route_param = match route_param(&request, 0) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
        let input = request.validated::<I>()?;
        self(route_param, Validated::new(input), request)
            .into_response()
    }
}
