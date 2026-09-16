use super::Router;
use crate::{controller::Handler, middleware::Layers, Method, Middleware, Result};
use std::sync::Arc;

/// Public route registrar backed by the application's internal router.
///
/// `Route` borrows the application router, so route registration stays explicit
/// and does not require global mutable state.
pub struct Route<'a> {
    router: &'a mut Router,
    prefixes: Vec<String>,
    layers: Layers,
}

impl<'a> Route<'a> {
    pub(crate) fn new(router: &'a mut Router) -> Self {
        Self {
            router,
            prefixes: Vec::new(),
            layers: Layers::default(),
        }
    }

    pub fn get<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        self.add(Method::new("GET")?, path, handler)
    }

    pub fn post<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        self.add(Method::new("POST")?, path, handler)
    }

    pub fn put<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        self.add(Method::new("PUT")?, path, handler)
    }

    pub fn patch<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        self.add(Method::new("PATCH")?, path, handler)
    }

    pub fn delete<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        self.add(Method::new("DELETE")?, path, handler)
    }

    /// Creates a nested route scope with an additional path prefix.
    ///
    /// The returned scope borrows this registrar and does not mutate the parent
    /// scope, so the prefix applies only to routes registered through it.
    pub fn prefix<'b>(&'b mut self, prefix: impl Into<String>) -> Route<'b> {
        let mut prefixes = self.prefixes.clone();
        prefixes.push(prefix.into());
        Route {
            router: self.router,
            prefixes,
            layers: self.layers.clone(),
        }
    }

    /// Creates a nested route scope with one additional middleware layer.
    ///
    /// Middleware is applied outside previously-added inner scope middleware.
    pub fn middleware<'b>(&'b mut self, layer: impl Middleware) -> Route<'b> {
        let mut layers = self.layers.clone();
        layers.0.push(Arc::new(layer));
        Route {
            router: self.router,
            prefixes: self.prefixes.clone(),
            layers,
        }
    }

    /// Registers a group atomically. If configuration fails, none of the
    /// routes created inside the group are added to the parent router.
    pub fn group(
        &mut self,
        configure: impl FnOnce(&mut Route<'_>) -> Result<()>,
    ) -> Result<()> {
        let mut child_router = Router::default();
        {
            let mut child = Route::new(&mut child_router);
            configure(&mut child)?;
        }
        self.mount_scoped(child_router)
    }

    fn add<H, A>(&mut self, method: Method, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        let mut child = Router::default();
        child.add(method, path, handler)?;
        self.mount_scoped(child)
    }

    fn mount_scoped(&mut self, mut child: Router) -> Result<()> {
        for prefix in self.prefixes.iter().rev() {
            let mut parent = Router::default();
            parent.mount(child, prefix, Layers::default())?;
            child = parent;
        }
        self.router.mount(child, "", self.layers.clone())
    }
}
