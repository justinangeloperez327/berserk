use super::{
    resource::{ApiResourceController, ResourceController},
    route::Pattern,
    RouteError, Router,
};
use crate::{controller::Handler, middleware::Layers, Method, Middleware, Request, Result};
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

/// One named route registration. Use `Route::name` to construct this helper.
pub struct NamedRoute<'route, 'router> {
    route: &'route mut Route<'router>,
    name: String,
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

    /// Names one route without changing the existing verb return types.
    ///
    /// ```ignore
    /// route.name("users.show").get("/users/{id}", UserController::show)?;
    /// ```
    pub fn name<'b>(&'b mut self, name: impl Into<String>) -> NamedRoute<'b, 'a> {
        NamedRoute {
            route: self,
            name: name.into(),
        }
    }

    /// Registers a full REST resource atomically.
    ///
    /// Generated actions: index, create, store, show, edit, update (PUT/PATCH), destroy.
    pub fn resource<C>(&mut self, path: &str, controller: C) -> Result<()>
    where
        C: ResourceController,
    {
        let name = self.resource_name(path)?;
        let controller = Arc::new(controller);
        let mut child = Router::default();
        stage_api_resource(&mut child, path, &name, Arc::clone(&controller))?;

        let current = Arc::clone(&controller);
        child.add_named(
            Method::new("GET")?,
            &format!("{path}/create"),
            move |request: Request| current.create(request),
            Some(format!("{name}.create")),
        )?;

        let current = Arc::clone(&controller);
        child.add_named(
            Method::new("GET")?,
            &format!("{path}/{{id}}/edit"),
            move |id: C::Id, request: Request| current.edit(id, request),
            Some(format!("{name}.edit")),
        )?;

        self.mount_scoped(child)
    }

    /// Registers an API-only REST resource atomically.
    ///
    /// Generated actions: index, store, show, update (PUT/PATCH), destroy.
    pub fn api_resource<C>(&mut self, path: &str, controller: C) -> Result<()>
    where
        C: ApiResourceController,
    {
        let name = self.resource_name(path)?;
        let controller = Arc::new(controller);
        let mut child = Router::default();
        stage_api_resource(&mut child, path, &name, controller)?;
        self.mount_scoped(child)
    }

    /// Registers a fallback used only when no route pattern matches.
    ///
    /// Fallbacks may use route middleware but cannot be registered beneath a
    /// path prefix because prefix-scoped fallback matching is not supported.
    pub fn fallback<H, A>(&mut self, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        let mut child = Router::default();
        child.set_fallback(handler)?;
        self.mount_scoped(child)
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
    /// Chained middleware runs in registration order.
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
    pub fn group(&mut self, configure: impl FnOnce(&mut Route<'_>) -> Result<()>) -> Result<()> {
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
        self.add_named(method, path, handler, None)
    }

    fn add_named<H, A>(
        &mut self,
        method: Method,
        path: &str,
        handler: H,
        name: Option<String>,
    ) -> Result<()>
    where
        H: Handler<A>,
    {
        let mut child = Router::default();
        child.add_named(method, path, handler, name)?;
        self.mount_scoped(child)
    }

    fn resource_name(&self, path: &str) -> Result<String> {
        if path == "/" || path.ends_with('/') {
            return Err(RouteError::InvalidResourcePath.into());
        }
        let mut pattern = Pattern::parse(path)?;
        if pattern.parameter_count() != 0 {
            return Err(RouteError::InvalidResourcePath.into());
        }
        for prefix in self.prefixes.iter().rev() {
            pattern = pattern.prefixed(prefix)?;
        }
        let name = pattern
            .source()
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>()
            .join(".");
        if name.is_empty() {
            return Err(RouteError::InvalidResourcePath.into());
        }
        Ok(name)
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

impl NamedRoute<'_, '_> {
    pub fn get<H, A>(self, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        self.route
            .add_named(Method::new("GET")?, path, handler, Some(self.name))
    }

    pub fn post<H, A>(self, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        self.route
            .add_named(Method::new("POST")?, path, handler, Some(self.name))
    }

    pub fn put<H, A>(self, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        self.route
            .add_named(Method::new("PUT")?, path, handler, Some(self.name))
    }

    pub fn patch<H, A>(self, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        self.route
            .add_named(Method::new("PATCH")?, path, handler, Some(self.name))
    }

    pub fn delete<H, A>(self, path: &str, handler: H) -> Result<()>
    where
        H: Handler<A>,
    {
        self.route
            .add_named(Method::new("DELETE")?, path, handler, Some(self.name))
    }
}

fn stage_api_resource<C>(
    router: &mut Router,
    path: &str,
    name: &str,
    controller: Arc<C>,
) -> Result<()>
where
    C: ApiResourceController,
{
    let member = format!("{path}/{{id}}");

    let current = Arc::clone(&controller);
    router.add_named(
        Method::new("GET")?,
        path,
        move |request: Request| current.index(request),
        Some(format!("{name}.index")),
    )?;

    let current = Arc::clone(&controller);
    router.add_named(
        Method::new("POST")?,
        path,
        move |request: Request| current.store(request),
        Some(format!("{name}.store")),
    )?;

    let current = Arc::clone(&controller);
    router.add_named(
        Method::new("GET")?,
        &member,
        move |id: C::Id, request: Request| current.show(id, request),
        Some(format!("{name}.show")),
    )?;

    let current = Arc::clone(&controller);
    router.add_named(
        Method::new("PUT")?,
        &member,
        move |id: C::Id, request: Request| current.update(id, request),
        Some(format!("{name}.update")),
    )?;

    let current = Arc::clone(&controller);
    router.add_named(
        Method::new("PATCH")?,
        &member,
        move |id: C::Id, request: Request| current.update(id, request),
        Some(format!("{name}.update")),
    )?;

    let current = Arc::clone(&controller);
    router.add_named(
        Method::new("DELETE")?,
        &member,
        move |id: C::Id, request: Request| current.destroy(id, request),
        Some(format!("{name}.destroy")),
    )?;

    Ok(())
}
