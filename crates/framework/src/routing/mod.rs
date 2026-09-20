//! Deterministic synchronous routing without network I/O.
mod error;
mod facade;
mod resource;
mod route;

use crate::{controller::Handler, Method, Request, Response, Result};
pub use error::RouteError;
pub use facade::{NamedRoute, Route};
pub use resource::{ApiResourceController, ResourceController};
use route::Pattern;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

type BoxedHandler = Box<dyn Fn(Request) -> Result<Response> + Send + Sync + 'static>;
struct RegisteredRoute {
    method: Method,
    pattern: Pattern,
    name: Option<String>,
    handler: BoxedHandler,
}

#[derive(Default)]
pub(crate) struct Router {
    routes: Vec<RegisteredRoute>,
    fallback: Option<BoxedHandler>,
}
impl fmt::Debug for Router {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router")
            .field("route_count", &self.routes.len())
            .field("has_fallback", &self.fallback.is_some())
            .finish()
    }
}

impl Router {
    pub(crate) fn add<H, A>(&mut self, method: Method, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        self.add_named(method, path, handler, None)
    }

    pub(crate) fn add_named<H, A>(
        &mut self,
        method: Method,
        path: &str,
        handler: H,
        name: Option<String>,
    ) -> Result<()>
    where
        H: Handler<A>,
    {
        let pattern = Pattern::parse(path)?;
        if let Some(name) = name.as_deref() {
            validate_name(name)?;
        }
        if let Some(expected) = H::expected_route_params() {
            let actual = pattern.parameter_count();
            if expected != actual {
                return Err(RouteError::ParameterCountMismatch { expected, actual }.into());
            }
        }
        if self
            .routes
            .iter()
            .any(|route| route.method == method && route.pattern.equivalent(&pattern))
        {
            return Err(RouteError::DuplicateRoute.into());
        }
        if let Some(name) = name.as_deref() {
            if let Some(existing) = self
                .routes
                .iter()
                .find(|route| route.name.as_deref() == Some(name))
            {
                if !existing.pattern.same_template(&pattern) {
                    return Err(RouteError::DuplicateRouteName(name.to_owned()).into());
                }
            }
        }
        self.routes.push(RegisteredRoute {
            method,
            pattern,
            name,
            handler: Box::new(move |request| call_handler(&handler, request)),
        });
        Ok(())
    }

    pub(crate) fn set_fallback<H, A>(&mut self, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        if let Some(expected) = H::expected_route_params() {
            if expected != 0 {
                return Err(RouteError::ParameterCountMismatch {
                    expected,
                    actual: 0,
                }
                .into());
            }
        }
        if self.fallback.is_some() {
            return Err(RouteError::DuplicateFallback.into());
        }
        self.fallback = Some(Box::new(move |request| call_handler(&handler, request)));
        Ok(())
    }

    pub(crate) fn path_for(&self, name: &str, params: &[(&str, &str)]) -> Result<String> {
        let route = self
            .routes
            .iter()
            .find(|route| route.name.as_deref() == Some(name))
            .ok_or_else(|| RouteError::UnknownRouteName(name.to_owned()))?;
        let values = params
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<BTreeMap<_, _>>();
        route.pattern.build(&values).map_err(Into::into)
    }

    pub(crate) fn dispatch(&self, mut request: Request) -> Result<Response> {
        let head = request.method().as_str() == "HEAD";
        let best = self
            .routes
            .iter()
            .filter(|route| route.pattern.captures(request.path()).is_some())
            .max_by_key(|route| route.pattern.specificity());
        let response = if let Some(best) = best {
            let candidates: Vec<_> = self
                .routes
                .iter()
                .filter(|route| route.pattern.equivalent(&best.pattern))
                .collect();
            let selected = candidates
                .iter()
                .copied()
                .find(|route| &route.method == request.method())
                .or_else(|| {
                    if head {
                        candidates
                            .iter()
                            .copied()
                            .find(|route| route.method.as_str() == "GET")
                    } else {
                        None
                    }
                });
            if let Some(route) = selected {
                let params = route
                    .pattern
                    .captures(request.path())
                    .expect("selected pattern matches");
                request.set_params(params);
                (route.handler)(request)?
            } else {
                let mut allow: BTreeSet<&str> = candidates
                    .iter()
                    .map(|route| route.method.as_str())
                    .collect();
                if allow.contains("GET") {
                    allow.insert("HEAD");
                }
                crate::Error::rejected(405, "Method Not Allowed")
                    .response()
                    .header("allow", &allow.into_iter().collect::<Vec<_>>().join(", "))?
            }
        } else if let Some(fallback) = &self.fallback {
            fallback(request)?
        } else {
            crate::Error::not_found().response()
        };
        crate::IntoResponse::into_response(response)
    }
}

impl Router {
    pub(crate) fn mount(
        &mut self,
        other: Router,
        prefix: &str,
        layers: crate::middleware::Layers,
    ) -> Result<()> {
        if !prefix.is_empty() {
            Pattern::parse(prefix)?;
        }

        let Router { routes, fallback } = other;

        if fallback.is_some() && !prefix.is_empty() {
            return Err(RouteError::ScopedFallback.into());
        }
        if fallback.is_some() && self.fallback.is_some() {
            return Err(RouteError::DuplicateFallback.into());
        }

        let mut pending = Vec::new();
        for route in routes {
            let pattern = route.pattern.prefixed(prefix)?;
            if self.routes.iter().chain(pending.iter()).any(|existing| {
                existing.method == route.method && existing.pattern.equivalent(&pattern)
            }) {
                return Err(RouteError::DuplicateRoute.into());
            }
            if let Some(name) = route.name.as_deref() {
                if let Some(existing) = self
                    .routes
                    .iter()
                    .chain(pending.iter())
                    .find(|existing| existing.name.as_deref() == Some(name))
                {
                    if !existing.pattern.same_template(&pattern) {
                        return Err(RouteError::DuplicateRouteName(name.to_owned()).into());
                    }
                }
            }
            let layers = layers.clone();
            let handler = route.handler;
            pending.push(RegisteredRoute {
                method: route.method,
                pattern,
                name: route.name,
                handler: Box::new(move |request| layers.run(request, handler.as_ref())),
            });
        }

        let pending_fallback = fallback.map(|handler| {
            let layers = layers.clone();
            Box::new(move |request| layers.run(request, handler.as_ref())) as BoxedHandler
        });

        self.routes.extend(pending);
        if let Some(fallback) = pending_fallback {
            self.fallback = Some(fallback);
        }
        Ok(())
    }
}

fn validate_name(name: &str) -> std::result::Result<(), RouteError> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(RouteError::InvalidRouteName);
    }
    Ok(())
}

fn call_handler<H: crate::controller::Handler<A>, A>(
    handler: &H,
    request: Request,
) -> Result<Response> {
    #[cfg(feature = "auth")]
    {
        crate::authorization::scope_principal(request.user().cloned(), || {
            handler.call(request)
        })
    }
    #[cfg(not(feature = "auth"))]
    {
        handler.call(request)
    }
}

#[cfg(feature = "claw")]
pub use resource::CrudController;
