use crate::{Result, ServerConfig, Validate};

/// Validated application configuration for in-memory routing and Tokio/Hyper HTTP serving.
#[derive(Debug)]
pub struct App {
    config: ServerConfig,
    layers: crate::middleware::Layers,
    state: crate::state::StateMap,
    router: crate::routing::Router,
}

impl App {
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
            router: Default::default(),
            layers: Default::default(),
            state: Default::default(),
        }
    }

    pub fn with_config(config: ServerConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            router: Default::default(),
            layers: Default::default(),
            state: Default::default(),
        })
    }

    /// Immutable access prevents configuration changes after validation.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

#[cfg(feature = "openapi")]
impl App {
    /// Registers the runtime route and its OpenAPI operation as one setup action.
    pub fn documented_route<H, A>(
        &mut self,
        document: &mut berserk_openapi::OpenApi,
        method: berserk_openapi::HttpMethod,
        path: &str,
        operation: berserk_openapi::Operation,
        handler: H,
    ) -> Result<()>
    where
        H: crate::controller::Handler<A>,
    {
        let mut staged = document.clone();
        staged.operation(method, path, operation)?;
        let runtime_method = match method {
            berserk_openapi::HttpMethod::Get => "GET",
            berserk_openapi::HttpMethod::Post => "POST",
            berserk_openapi::HttpMethod::Put => "PUT",
            berserk_openapi::HttpMethod::Patch => "PATCH",
            berserk_openapi::HttpMethod::Delete => "DELETE",
            berserk_openapi::HttpMethod::Head => "HEAD",
            berserk_openapi::HttpMethod::Options => "OPTIONS",
        };
        self.register_route(crate::Method::new(runtime_method)?, path, handler)?;
        *document = staged;
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    fn register_route<H, A>(&mut self, method: crate::Method, path: &str, handler: H) -> Result<()>
    where
        H: crate::controller::Handler<A>,
    {
        self.router.add(method, path, handler)
    }

    /// Returns an instance route registrar borrowing this application.
    pub fn route(&mut self) -> crate::routing::Route<'_> {
        crate::routing::Route::new(&mut self.router)
    }

    /// Builds a path for a named route, percent-encoding route parameter values.
    pub fn path_for(&self, name: &str, params: &[(&str, &str)]) -> Result<String> {
        self.router.path_for(name, params)
    }

    /// Dispatch without network I/O. Handler errors propagate; panics are not caught here.
    pub fn handle(&self, request: crate::Request) -> Result<crate::Response> {
        let mut request = request;
        let head = request.method().as_str() == "HEAD";
        request.set_state(self.state.clone());
        let terminal = |request| self.router.dispatch(request);
        #[cfg(feature = "database")]
        let result = request
            .database_scope()
            .run(|| self.layers.run(request, &terminal));
        #[cfg(not(feature = "database"))]
        let result = self.layers.run(request, &terminal);
        let mut response = result?;
        response.validate()?;
        if head {
            response.suppress_for_head();
        }
        Ok(response)
    }

    /// Compatibility shortcut. Prefer `app.route().get(...)` for route registration.
    pub fn get<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        H: crate::controller::Handler<A>,
    {
        self.register_route(crate::Method::new("GET")?, path, handler)
    }

    /// Compatibility shortcut. Prefer `app.route().post(...)` for route registration.
    pub fn post<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        H: crate::controller::Handler<A>,
    {
        self.register_route(crate::Method::new("POST")?, path, handler)
    }

    /// Compatibility shortcut. Prefer `app.route().put(...)` for route registration.
    pub fn put<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        H: crate::controller::Handler<A>,
    {
        self.register_route(crate::Method::new("PUT")?, path, handler)
    }

    /// Compatibility shortcut. Prefer `app.route().patch(...)` for route registration.
    pub fn patch<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        H: crate::controller::Handler<A>,
    {
        self.register_route(crate::Method::new("PATCH")?, path, handler)
    }

    /// Compatibility shortcut. Prefer `app.route().delete(...)` for route registration.
    pub fn delete<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        H: crate::controller::Handler<A>,
    {
        self.register_route(crate::Method::new("DELETE")?, path, handler)
    }
}

#[cfg(feature = "server")]
impl App {
    pub fn bind(self, address: impl std::net::ToSocketAddrs) -> Result<crate::server::Server> {
        Ok(crate::server::Server::bind(self, address)?)
    }

    pub fn listen(self, address: impl std::net::ToSocketAddrs) -> Result<()> {
        self.bind(address)?.run()?;
        Ok(())
    }
}

impl App {
    pub fn middleware(&mut self, layer: impl crate::Middleware) {
        self.layers.0.push(std::sync::Arc::new(layer));
    }

    /// One value per type. Newtype wrappers distinguish values of the same underlying type.
    pub fn state<T: Send + Sync + 'static>(&mut self, value: T) -> Result<()> {
        if !self.state.insert(value) {
            return Err(crate::ConfigError::new("state", "type already registered").into());
        }
        Ok(())
    }

    /// Validates typed application configuration once, before any request is served.
    pub fn configure<T: Validate + Send + Sync + 'static>(&mut self, value: T) -> Result<()> {
        value.validate()?;
        if !self.state.insert(crate::config::Configuration(value)) {
            return Err(crate::ConfigError::new("config", "type already configured").into());
        }
        Ok(())
    }

    #[cfg(feature = "database")]
    pub fn database(&mut self, database: berserk_database::Database) -> Result<()> {
        self.state(database)
    }

    /// Configure the authentication backend used by route-level auth helpers.
    #[cfg(feature = "auth")]
    pub fn auth<G: berserk_auth::Guard>(&mut self, guard: G) -> Result<()> {
        if !self
            .state
            .insert(crate::middleware::ConfiguredAuth::new(guard))
        {
            return Err(
                crate::ConfigError::new("auth", "authentication is already configured").into(),
            );
        }
        Ok(())
    }

    /// Build routes transactionally. The child application's config and state are not inherited.
    pub fn group(
        &mut self,
        prefix: &str,
        configure: impl FnOnce(&mut App) -> Result<()>,
    ) -> Result<()> {
        let mut child = App::new();
        configure(&mut child)?;
        if !child.state.is_empty() {
            return Err(
                crate::ConfigError::new("group", "register state on the root application").into(),
            );
        }
        self.router.mount(child.router, prefix, child.layers)
    }
}

impl App {
    /// Dispatch and render public HTTP errors, as the test client does.
    pub fn respond(&self, request: crate::Request) -> crate::Response {
        let head = request.method().as_str() == "HEAD";
        let mut response = self
            .handle(request)
            .unwrap_or_else(|error| error.response());
        if head {
            response.suppress_for_head();
        }
        response
    }
}

#[cfg(feature = "async")]
impl App {
    /// Dispatch on a blocking worker. Dropping this future does not cancel a started action.
    pub async fn handle_async(
        self: &std::sync::Arc<Self>,
        request: crate::Request,
    ) -> Result<crate::Response> {
        let app = self.clone();
        tokio::task::spawn_blocking(move || {
            crate::controller::async_handlers::in_worker(|| app.handle(request))
        })
        .await
        .map_err(|_| crate::ConfigError::new("async", "request worker failed"))?
    }
}

#[cfg(feature = "async")]
impl App {
    pub fn get_async<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        crate::controller::async_handlers::Async<H>: crate::controller::Handler<A>,
    {
        self.route().get_async(path, handler)
    }
}

#[cfg(feature = "async")]
impl App {
    pub fn post_async<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        crate::controller::async_handlers::Async<H>: crate::controller::Handler<A>,
    {
        self.route().post_async(path, handler)
    }
}

#[cfg(feature = "async")]
impl App {
    pub fn put_async<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        crate::controller::async_handlers::Async<H>: crate::controller::Handler<A>,
    {
        self.route().put_async(path, handler)
    }
}

#[cfg(feature = "async")]
impl App {
    pub fn patch_async<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        crate::controller::async_handlers::Async<H>: crate::controller::Handler<A>,
    {
        self.route().patch_async(path, handler)
    }
}

#[cfg(feature = "async")]
impl App {
    pub fn delete_async<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        crate::controller::async_handlers::Async<H>: crate::controller::Handler<A>,
    {
        self.route().delete_async(path, handler)
    }
}

#[cfg(feature = "async")]
impl App {
    pub fn head_async<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        crate::controller::async_handlers::Async<H>: crate::controller::Handler<A>,
    {
        self.route().head_async(path, handler)
    }
}

#[cfg(feature = "async")]
impl App {
    pub fn options_async<H, A>(&mut self, path: &str, handler: H) -> Result<()>
    where
        crate::controller::async_handlers::Async<H>: crate::controller::Handler<A>,
    {
        self.route().options_async(path, handler)
    }
}
