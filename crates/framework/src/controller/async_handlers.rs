//! Optional async actions execute on a blocking worker, with one stable request scope.
use super::*;
use std::{cell::Cell, future::Future, sync::Arc};

pub struct Async<F>(pub(crate) Arc<F>);
pub struct AsyncArgs<A>(PhantomData<A>);
thread_local! { static WORKER: Cell<bool> = const { Cell::new(false) }; }
pub(crate) fn in_worker<T>(operation: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            WORKER.with(|w| w.set(self.0));
        }
    }
    let _restore = Restore(WORKER.with(|w| w.replace(true)));
    operation()
}
struct Await<F>(F);
impl<F: Future> IntoResponse for Await<F>
where
    F::Output: IntoResponse,
{
    fn into_response(self) -> Result<Response> {
        let result = match tokio::runtime::Handle::try_current() {
            Ok(handle) if WORKER.with(Cell::get) => handle.block_on(self.0),
            Ok(handle)
                if matches!(
                    handle.runtime_flavor(),
                    tokio::runtime::RuntimeFlavor::MultiThread
                ) =>
            {
                tokio::task::block_in_place(|| handle.block_on(self.0))
            }
            Ok(_) => {
                return Err(crate::ConfigError::new(
                    "async",
                    "use App::handle_async from a current-thread runtime",
                )
                .into())
            }
            Err(_) => tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()?
                .block_on(self.0),
        };
        result.into_response()
    }
}
macro_rules! adapter {
    ([$($generic:ident),*], $marker:ty, ($($name:ident : $arg:ty),*), [$($bounds:tt)*], $count:expr) => {
        impl<F, Fu, R, $($generic,)*> Handler<AsyncArgs<$marker>> for Async<F>
        where F: Fn($($arg),*) -> Fu + Send + Sync + 'static,
            Fu: Future<Output=R>, R: IntoResponse, $($bounds)*
        {
            fn expected_route_params() -> Option<usize> { $count }
            fn call(&self, request: Request) -> Result<Response> {
                let action = Arc::clone(&self.0);
                let wrapped = move |$($name: $arg),*| Await(action($($name),*));
                <_ as Handler<$marker>>::call(&wrapped, request)
            }
        }
    };
}

adapter!([], NoArgs, (), [], Some(0));
adapter!([], RequestArg, (arg0: Request), [], None);
adapter!([T], RouteParam<T>, (arg0: T), [T: FromStr + 'static,], Some(1));
adapter!([T], RouteParamRequest<T>, (arg0: T, arg1: Request), [T: FromStr + 'static,], Some(1));
#[cfg(feature = "claw")]
adapter!([M], RouteModel<M>, (arg0: M), [M: claw_orm::Model + 'static,], Some(1));
#[cfg(feature = "claw")]
adapter!([M], RouteModelRequest<M>, (arg0: M, arg1: Request), [M: claw_orm::Model + 'static,], Some(1));
#[cfg(feature = "claw")]
adapter!([M, N], RouteModels<M, N>, (arg0: M, arg1: N), [M: claw_orm::Model + 'static,
    N: claw_orm::ScopedRouteModel<M> + 'static,], Some(2));
#[cfg(feature = "claw")]
adapter!([M, N], RouteModelsRequest<M, N>, (arg0: M, arg1: N, arg2: Request), [M: claw_orm::Model + 'static,
    N: claw_orm::ScopedRouteModel<M> + 'static,], Some(2));
#[cfg(feature = "claw")]
adapter!([M, N, I], RouteModelsValidated<M, N, I>, (arg0: M, arg1: N, arg2: Validated<I>), [M: claw_orm::Model + 'static,
    N: claw_orm::ScopedRouteModel<M> + 'static,
    I: FromJson + ValidateInput + 'static,], Some(2));
#[cfg(feature = "claw")]
adapter!([M, N, I], RouteModelsValidatedRequest<M, N, I>, (arg0: M, arg1: N, arg2: Validated<I>, arg3: Request), [M: claw_orm::Model + 'static,
    N: claw_orm::ScopedRouteModel<M> + 'static,
    I: FromJson + ValidateInput + 'static,], Some(2));
#[cfg(feature = "claw")]
adapter!([M, I], RouteModelValidated<M, I>, (arg0: M, arg1: Validated<I>), [M: claw_orm::Model + 'static,
    I: FromJson + ValidateInput + 'static,], Some(1));
#[cfg(feature = "claw")]
adapter!([M, I], RouteModelValidatedRequest<M, I>, (arg0: M, arg1: Validated<I>, arg2: Request), [M: claw_orm::Model + 'static,
    I: FromJson + ValidateInput + 'static,], Some(1));
adapter!([T, U], RouteParams<T, U>, (arg0: T, arg1: U), [T: FromStr + 'static,
    U: FromStr + 'static,], Some(2));
adapter!([T, U], RouteParamsRequest<T, U>, (arg0: T, arg1: U, arg2: Request), [T: FromStr + 'static,
    U: FromStr + 'static,], Some(2));
adapter!([T, U, I], RouteParamsValidated<T, U, I>, (arg0: T, arg1: U, arg2: Validated<I>), [T: FromStr + 'static,
    U: FromStr + 'static,
    I: FromJson + ValidateInput + 'static,], Some(2));
adapter!([T, U, I], RouteParamsValidatedRequest<T, U, I>, (arg0: T, arg1: U, arg2: Validated<I>, arg3: Request), [T: FromStr + 'static,
    U: FromStr + 'static,
    I: FromJson + ValidateInput + 'static,], Some(2));
adapter!([T], ValidatedArg<T>, (arg0: Validated<T>), [T: FromJson + ValidateInput + 'static,], Some(0));
adapter!([T], ValidatedRequest<T>, (arg0: Validated<T>, arg1: Request), [T: FromJson + ValidateInput + 'static,], Some(0));
adapter!([T, I], RouteParamValidated<T, I>, (arg0: T, arg1: Validated<I>), [T: FromStr + 'static,
    I: FromJson + ValidateInput + 'static,], Some(1));
adapter!([T, I], RouteParamValidatedRequest<T, I>, (arg0: T, arg1: Validated<I>, arg2: Request), [T: FromStr + 'static,
    I: FromJson + ValidateInput + 'static,], Some(1));
#[cfg(feature = "claw")]
adapter!([M, N, I], RouteModelsForm<M, N, I>, (arg0: M, arg1: N, arg2: I), [M: claw_orm::Model + 'static,
    N: claw_orm::ScopedRouteModel<M> + 'static,
    I: FormRequest + 'static,], Some(2));
#[cfg(feature = "claw")]
adapter!([M, N, I], RouteModelsFormAndRequest<M, N, I>, (arg0: M, arg1: N, arg2: I, arg3: Request), [M: claw_orm::Model + 'static,
    N: claw_orm::ScopedRouteModel<M> + 'static,
    I: FormRequest + 'static,], Some(2));
#[cfg(feature = "claw")]
adapter!([M, I], RouteModelForm<M, I>, (arg0: M, arg1: I), [M: claw_orm::Model + 'static,
    I: FormRequest + 'static,], Some(1));
#[cfg(feature = "claw")]
adapter!([M, I], RouteModelFormAndRequest<M, I>, (arg0: M, arg1: I, arg2: Request), [M: claw_orm::Model + 'static,
    I: FormRequest + 'static,], Some(1));
adapter!([T, U, I], RouteParamsForm<T, U, I>, (arg0: T, arg1: U, arg2: I), [T: FromStr + 'static,
    U: FromStr + 'static,
    I: FormRequest + 'static,], Some(2));
adapter!([T, U, I], RouteParamsFormAndRequest<T, U, I>, (arg0: T, arg1: U, arg2: I, arg3: Request), [T: FromStr + 'static,
    U: FromStr + 'static,
    I: FormRequest + 'static,], Some(2));
adapter!([T], FormArg<T>, (arg0: T), [T: FormRequest + 'static,], Some(0));
adapter!([T], FormAndRequest<T>, (arg0: T, arg1: Request), [T: FormRequest + 'static,], Some(0));
adapter!([T, I], RouteParamForm<T, I>, (arg0: T, arg1: I), [T: FromStr + 'static,
    I: FormRequest + 'static,], Some(1));
adapter!([T, I], RouteParamFormAndRequest<T, I>, (arg0: T, arg1: I, arg2: Request), [T: FromStr + 'static,
    I: FormRequest + 'static,], Some(1));
