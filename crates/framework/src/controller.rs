//! Controller-oriented handler adapters.
use crate::{FromJson, IntoResponse, Request, Response, Result, ValidateInput, Validated};
use std::{marker::PhantomData, str::FromStr};

/// Conventional result type for controller actions that can fail.
pub type ActionResult = Result<Response>;
type Extracted<T> = std::result::Result<T, Response>;

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

#[cfg(feature = "claw")]
#[doc(hidden)]
pub struct RouteModels<M, N>(PhantomData<M>, PhantomData<N>);

#[cfg(feature = "claw")]
#[doc(hidden)]
pub struct RouteModelsRequest<M, N>(PhantomData<M>, PhantomData<N>);

#[cfg(feature = "claw")]
#[doc(hidden)]
pub struct RouteModelsValidated<M, N, I>(PhantomData<M>, PhantomData<N>, PhantomData<I>);

#[cfg(feature = "claw")]
#[doc(hidden)]
pub struct RouteModelsValidatedRequest<M, N, I>(PhantomData<M>, PhantomData<N>, PhantomData<I>);

#[cfg(feature = "claw")]
#[doc(hidden)]
pub struct RouteModelValidated<M, I>(PhantomData<M>, PhantomData<I>);

#[cfg(feature = "claw")]
#[doc(hidden)]
pub struct RouteModelValidatedRequest<M, I>(PhantomData<M>, PhantomData<I>);

#[doc(hidden)]
pub trait Handler<Args>: Send + Sync + 'static {
    fn expected_route_params() -> Option<usize>;
    fn call(&self, request: Request) -> Result<Response>;
}

fn route_param<T: FromStr>(request: &Request, index: usize) -> Extracted<T> {
    match request.param_at_as::<T>(index) {
        Some(Ok(value)) => Ok(value),
        Some(Err(_)) | None => Err(Response::text("Invalid route parameter").status(400)),
    }
}

#[cfg(feature = "claw")]
fn route_model_key<M: claw_orm::Model>(
    request: &Request,
    index: usize,
) -> Extracted<framework_database::Value> {
    let raw = match request.param_at(index) {
        Some(value) => value,
        None => return Err(Response::text("Invalid route parameter").status(400)),
    };
    M::parse_route_key(raw).ok_or_else(|| Response::text("Invalid route parameter").status(400))
}

#[cfg(feature = "claw")]
fn route_database(request: &Request) -> Result<&framework_database::Database> {
    request
        .state::<framework_database::Database>()
        .ok_or_else(|| {
            framework_core::ConfigError::new("database", "database state is not configured").into()
        })
}

#[cfg(feature = "claw")]
fn route_model<M: claw_orm::Model>(request: &Request, index: usize) -> Result<Extracted<M>> {
    let key = match route_model_key::<M>(request, index) {
        Ok(key) => key,
        Err(response) => return Ok(Err(response)),
    };
    let mut connection = route_database(request)?.acquire()?;
    match M::find(&mut *connection, key)? {
        Some(model) => Ok(Ok(model)),
        None => Ok(Err(Response::text("Not Found").status(404))),
    }
}

#[cfg(feature = "claw")]
fn route_models<M, N>(request: &Request) -> Result<Extracted<(M, N)>>
where
    M: claw_orm::Model,
    N: claw_orm::Model,
{
    let first_key = match route_model_key::<M>(request, 0) {
        Ok(key) => key,
        Err(response) => return Ok(Err(response)),
    };
    let second_key = match route_model_key::<N>(request, 1) {
        Ok(key) => key,
        Err(response) => return Ok(Err(response)),
    };
    let mut connection = route_database(request)?.acquire()?;
    let first = match M::find(&mut *connection, first_key)? {
        Some(model) => model,
        None => return Ok(Err(Response::text("Not Found").status(404))),
    };
    let second = match N::find(&mut *connection, second_key)? {
        Some(model) => model,
        None => return Ok(Err(Response::text("Not Found").status(404))),
    };
    Ok(Ok((first, second)))
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

#[cfg(feature = "claw")]
impl<F, M, N, R> Handler<RouteModels<M, N>> for F
where
    F: Fn(M, N) -> R + Send + Sync + 'static,
    M: claw_orm::Model + 'static,
    N: claw_orm::Model + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(2)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let (first, second) = match route_models::<M, N>(&request)? {
            Ok(models) => models,
            Err(response) => return Ok(response),
        };
        self(first, second).into_response()
    }
}

#[cfg(feature = "claw")]
impl<F, M, N, R> Handler<RouteModelsRequest<M, N>> for F
where
    F: Fn(M, N, Request) -> R + Send + Sync + 'static,
    M: claw_orm::Model + 'static,
    N: claw_orm::Model + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(2)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let (first, second) = match route_models::<M, N>(&request)? {
            Ok(models) => models,
            Err(response) => return Ok(response),
        };
        self(first, second, request).into_response()
    }
}

#[cfg(feature = "claw")]
impl<F, M, N, I, R> Handler<RouteModelsValidated<M, N, I>> for F
where
    F: Fn(M, N, Validated<I>) -> R + Send + Sync + 'static,
    M: claw_orm::Model + 'static,
    N: claw_orm::Model + 'static,
    I: FromJson + ValidateInput + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(2)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let (first, second) = match route_models::<M, N>(&request)? {
            Ok(models) => models,
            Err(response) => return Ok(response),
        };
        let input = request.validated::<I>()?;
        self(first, second, Validated::new(input)).into_response()
    }
}

#[cfg(feature = "claw")]
impl<F, M, N, I, R> Handler<RouteModelsValidatedRequest<M, N, I>> for F
where
    F: Fn(M, N, Validated<I>, Request) -> R + Send + Sync + 'static,
    M: claw_orm::Model + 'static,
    N: claw_orm::Model + 'static,
    I: FromJson + ValidateInput + 'static,
    R: IntoResponse,
{
    fn expected_route_params() -> Option<usize> {
        Some(2)
    }

    fn call(&self, request: Request) -> Result<Response> {
        let (first, second) = match route_models::<M, N>(&request)? {
            Ok(models) => models,
            Err(response) => return Ok(response),
        };
        let input = request.validated::<I>()?;
        self(first, second, Validated::new(input), request)
            .into_response()
    }
}

#[cfg(feature = "claw")]
impl<F, M, I, R> Handler<RouteModelValidated<M, I>> for F
where
    F: Fn(M, Validated<I>) -> R + Send + Sync + 'static,
    M: claw_orm::Model + 'static,
    I: FromJson + ValidateInput + 'static,
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
        let input = request.validated::<I>()?;
        self(model, Validated::new(input)).into_response()
    }
}

#[cfg(feature = "claw")]
impl<F, M, I, R> Handler<RouteModelValidatedRequest<M, I>> for F
where
    F: Fn(M, Validated<I>, Request) -> R + Send + Sync + 'static,
    M: claw_orm::Model + 'static,
    I: FromJson + ValidateInput + 'static,
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
        let input = request.validated::<I>()?;
        self(model, Validated::new(input), request).into_response()
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
        self(first, second, Validated::new(input), request).into_response()
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
        self(route_param, Validated::new(input), request).into_response()
    }
}
