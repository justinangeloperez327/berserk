use crate::{Result, ServerConfig, Validate};
use std::str::FromStr;

/// Validated application configuration. Supports in-memory routing and synchronous TCP serving.
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
    pub fn documented_route<F, R>(
        &mut self,
        document: &mut framework_openapi::OpenApi,
        method: framework_openapi::HttpMethod,
        path: &str,
        operation: framework_openapi::Operation,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(crate::Request) -> R + Send + Sync + 'static,
        R: crate::IntoResponse,
    {
        let mut staged = document.clone();
        staged.operation(method, path, operation)?;
        let runtime_method = match method {
            framework_openapi::HttpMethod::Get => "GET",
            framework_openapi::HttpMethod::Post => "POST",
            framework_openapi::HttpMethod::Put => "PUT",
            framework_openapi::HttpMethod::Patch => "PATCH",
            framework_openapi::HttpMethod::Delete => "DELETE",
            framework_openapi::HttpMethod::Head => "HEAD",
            framework_openapi::HttpMethod::Options => "OPTIONS",
        };
        self.route(crate::Method::new(runtime_method)?, path, handler)?;
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
    pub fn route<F, R>(&mut self, method: crate::Method, path: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::Request) -> R + Send + Sync + 'static,
        R: crate::IntoResponse,
    {
        self.router.add(method, path, handler)
    }

    /// Register a controller action that receives one typed route parameter directly.
    ///
    /// This supports actions such as `fn show(id: u64) -> Response` without forcing
    /// the controller to accept the entire request.
    pub fn route_param<T, F, R>(
        &mut self,
        method: crate::Method,
        path: &str,
        parameter: &str,
        handler: F,
    ) -> Result<()>
    where
        T: FromStr + 'static,
        F: Fn(T) -> R + Send + Sync + 'static,
        R: crate::IntoResponse,
    {
        self.route(
            method,
            path,
            crate::controller::with_param::<T, F, R>(parameter.to_owned(), handler),
        )
    }

    /// Dispatch without network I/O. Handler errors propagate; panics are not caught here.
    pub fn handle(&self, request: crate::Request) -> Result<crate::Response> {
        let mut request = request;
        let head = request.method().as_str() == "HEAD";
        request.set_state(self.state.clone());
        let terminal = |request| self.router.dispatch(request);
        let mut response = self.layers.run(request, &terminal)?;
        response.validate()?;
        if head {
            response.suppress_for_head();
        }
        Ok(response)
    }

    pub fn get<F, R>(&mut self, path: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::Request) -> R + Send + Sync + 'static,
        R: crate::IntoResponse,
    {
        self.route(crate::Method::new("GET")?, path, handler)
    }

    pub fn get_param<T, F, R>(&mut self, path: &str, parameter: &str, handler: F) -> Result<()>
    where
        T: FromStr + 'static,
        F: Fn(T) -> R + Send + Sync + 'static,
        R: crate::IntoResponse,
    {
        self.route_param(crate::Method::new("GET")?, path, parameter, handler)
    }

    pub fn post<F, R>(&mut self, path: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::Request) -> R + Send + Sync + 'static,
        R: crate::IntoResponse,
    {
        self.route(crate::Method::new("POST")?, path, handler)
    }

    pub fn put<F, R>(&mut self, path: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::Request) -> R + Send + Sync + 'static,
        R: crate::IntoResponse,
    {
        self.route(crate::Method::new("PUT")?, path, handler)
    }

    pub fn patch<F, R>(&mut self, path: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::Request) -> R + Send + Sync + 'static,
        R: crate::IntoResponse,
    {
        self.route(crate::Method::new("PATCH")?, path, handler)
    }

    pub fn delete<F, R>(&mut self, path: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::Request) -> R + Send + Sync + 'static,
        R: crate::IntoResponse,
    {
        self.route(crate::Method::new("DELETE")?, path, handler)
    }

    pub fn head<F, R>(&mut self, path: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::Request) -> R + Send + Sync + 'static,
        R: crate::IntoResponse,
    {
        self.route(crate::Method::new("HEAD")?, path, handler)
    }

    pub fn options<F, R>(&mut self, path: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::Request) -> R + Send + Sync + 'static,
        R: crate::IntoResponse,
    {
        self.route(crate::Method::new("OPTIONS")?, path, handler)
    }
}

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
