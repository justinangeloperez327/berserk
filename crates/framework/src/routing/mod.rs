//! Deterministic synchronous routing without network I/O.
mod error;
mod route;

use crate::{IntoResponse, Method, Request, Response, Result};
pub use error::RouteError;
use route::Pattern;
use std::{collections::BTreeSet, fmt};

type Handler = Box<dyn Fn(Request) -> Result<Response> + Send + Sync + 'static>;
struct Route {
    method: Method,
    pattern: Pattern,
    handler: Handler,
}

#[derive(Default)]
pub(crate) struct Router {
    routes: Vec<Route>,
}
impl fmt::Debug for Router {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router")
            .field("route_count", &self.routes.len())
            .finish()
    }
}

impl Router {
    pub(crate) fn add<F, R>(&mut self, method: Method, path: &str, handler: F) -> Result<()>
    where
        F: Fn(Request) -> R + Send + Sync + 'static,
        R: IntoResponse,
    {
        let pattern = Pattern::parse(path)?;
        if self
            .routes
            .iter()
            .any(|r| r.method == method && r.pattern.equivalent(&pattern))
        {
            return Err(RouteError::DuplicateRoute.into());
        }
        self.routes.push(Route {
            method,
            pattern,
            handler: Box::new(move |request| handler(request).into_response()),
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
                .filter(|r| r.pattern.equivalent(&best.pattern))
                .collect();
            let selected = candidates
                .iter()
                .copied()
                .find(|r| &r.method == request.method())
                .or_else(|| {
                    if head {
                        candidates
                            .iter()
                            .copied()
                            .find(|r| r.method.as_str() == "GET")
                    } else {
                        None
                    }
                });
            if let Some(route) = selected {
                // Capture from the chosen method's pattern; parameter names may differ.
                let params = route
                    .pattern
                    .captures(request.path())
                    .expect("selected pattern matches");
                request.set_params(params);
                (route.handler)(request)?
            } else {
                let mut allow: BTreeSet<&str> =
                    candidates.iter().map(|r| r.method.as_str()).collect();
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
        response.into_response()
    }
}

impl Router {
    pub(crate) fn mount(
        &mut self,
        other: Router,
        prefix: &str,
        layers: crate::middleware::Layers,
    ) -> Result<()> {
        // Validate the prefix even if the child has no routes.
        if !prefix.is_empty() {
            Pattern::parse(prefix)?;
        }
        let mut pending = Vec::new();
        for route in other.routes {
            let pattern = route.pattern.prefixed(prefix)?;
            if self
                .routes
                .iter()
                .any(|r| r.method == route.method && r.pattern.equivalent(&pattern))
            {
                return Err(RouteError::DuplicateRoute.into());
            }
            let layers = layers.clone();
            let handler = route.handler;
            pending.push(Route {
                method: route.method,
                pattern,
                handler: Box::new(move |req| layers.run(req, handler.as_ref())),
            });
        }
        self.routes.extend(pending);
        Ok(())
    }
}
