//! Deterministic synchronous routing without network I/O.
mod error;
mod facade;
mod route;

use crate::{controller::Handler, Method, Request, Response, Result};
pub use error::RouteError;
pub use facade::Route;
use route::Pattern;
use std::{collections::BTreeSet, fmt};

type BoxedHandler = Box<dyn Fn(Request) -> Result<Response> + Send + Sync + 'static>;
struct RegisteredRoute {
    method: Method,
    pattern: Pattern,
    handler: BoxedHandler,
}

#[derive(Default)]
pub(crate) struct Router {
    routes: Vec<RegisteredRoute>,
}
impl fmt::Debug for Router {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router")
            .field("route_count", &self.routes.len())
            .finish()
    }
}

impl Router {
    pub(crate) fn add<H, A>(&mut self, method: Method, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        let pattern = Pattern::parse(path)?;
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
        self.routes.push(RegisteredRoute {
            method,
            pattern,
            handler: Box::new(move |request| handler.call(request)),
        });
        Ok(())
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
                Response::text("Method Not Allowed")
                    .status(405)
                    .header("allow", &allow.into_iter().collect::<Vec<_>>().join(", "))?
            }
        } else {
            Response::text("Not Found").status(404)
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
        let mut pending = Vec::new();
        for route in other.routes {
            let pattern = route.pattern.prefixed(prefix)?;
            if self.routes.iter().any(|existing| {
                existing.method == route.method && existing.pattern.equivalent(&pattern)
            }) {
                return Err(RouteError::DuplicateRoute.into());
            }
            let layers = layers.clone();
            let handler = route.handler;
            pending.push(RegisteredRoute {
                method: route.method,
                pattern,
                handler: Box::new(move |request| layers.run(request, handler.as_ref())),
            });
        }
        self.routes.extend(pending);
        Ok(())
    }
}
