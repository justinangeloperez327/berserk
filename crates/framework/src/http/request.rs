use super::{Headers, HttpError, Method};
use std::{collections::HashMap, str::FromStr};

#[cfg(feature = "database")]
use std::{
    cell::{RefCell, RefMut},
    ops::{Deref, DerefMut},
};

/// A mutable borrow of the database connection owned by one request.
#[cfg(feature = "database")]
pub struct RequestConnection<'a> {
    inner: RefMut<'a, Option<Box<dyn framework_database::Connection + Send>>>,
}

#[cfg(feature = "database")]
impl Deref for RequestConnection<'_> {
    type Target = dyn framework_database::Connection;

    fn deref(&self) -> &Self::Target {
        self.inner
            .as_deref()
            .expect("request connection is initialized before the guard is returned")
    }
}

#[cfg(feature = "database")]
impl DerefMut for RequestConnection<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
            .as_deref_mut()
            .expect("request connection is initialized before the guard is returned")
    }
}

#[cfg(feature = "database")]
struct RequestTransactionConnection<'a> {
    transaction: &'a mut dyn framework_database::Transaction,
    driver: framework_database::Driver,
    capabilities: framework_database::Capabilities,
}

#[cfg(feature = "database")]
impl framework_database::Connection for RequestTransactionConnection<'_> {
    fn driver(&self) -> framework_database::Driver {
        self.driver
    }

    fn capabilities(&self) -> framework_database::Capabilities {
        self.capabilities
    }

    fn execute(
        &mut self,
        statement: &framework_database::Statement,
    ) -> framework_database::Result<framework_database::Execution> {
        self.transaction.execute(statement)
    }

    fn query(
        &mut self,
        statement: &framework_database::Statement,
    ) -> framework_database::Result<Vec<framework_database::Row>> {
        self.transaction.query(statement)
    }

    fn begin(
        &mut self,
        _options: framework_database::TransactionOptions,
    ) -> framework_database::Result<Box<dyn framework_database::Transaction + '_>> {
        Err(framework_database::DatabaseError::new(
            framework_database::ErrorKind::Transaction,
            "nested transactions are not supported by request transactions",
        ))
    }

    fn ping(&mut self) -> framework_database::Result<()> {
        Err(framework_database::DatabaseError::new(
            framework_database::ErrorKind::Transaction,
            "ping is not available inside a request transaction",
        ))
    }
}

/// Owned request data. This constructor is not a wire parser.
pub struct Request {
    method: Method,
    state: crate::state::StateMap,
    request_id: Option<String>,
    trace_context: Option<crate::operational::TraceContext>,
    target: String,
    headers: Headers,
    body: Vec<u8>,
    params: HashMap<String, String>,
    param_values: Vec<String>,
    #[cfg(feature = "database")]
    connection: RefCell<Option<Box<dyn framework_database::Connection + Send>>>,
    #[cfg(feature = "auth")]
    principal: Option<framework_auth::Principal>,
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.method)
            .field("path", &self.path())
            .field("header_count", &self.headers.len())
            .field("body_bytes", &self.body.len())
            .field("parameter_count", &self.params.len())
            .field("request_id", &self.request_id)
            .finish_non_exhaustive()
    }
}

fn valid_target(target: &str) -> bool {
    if !target.starts_with('/') {
        return false;
    }
    let bytes = target.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' {
            if i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit()
            {
                return false;
            }
            i += 3;
            continue;
        }
        if !(b.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@/?".contains(&b)) {
            return false;
        }
        i += 1;
    }
    true
}

impl Request {
    pub fn new(
        method: Method,
        target: impl Into<String>,
        headers: Headers,
        body: impl Into<Vec<u8>>,
    ) -> Result<Self, HttpError> {
        let target = target.into();
        if !valid_target(&target) {
            return Err(HttpError::InvalidTarget);
        }
        Ok(Self {
            method,
            target,
            headers,
            body: body.into(),
            params: HashMap::new(),
            param_values: Vec::new(),
            state: Default::default(),
            request_id: None,
            trace_context: None,
            #[cfg(feature = "database")]
            connection: RefCell::new(None),
            #[cfg(feature = "auth")]
            principal: None,
        })
    }

    pub(crate) fn set_params(&mut self, params: Vec<(String, String)>) {
        self.params.clear();
        self.param_values.clear();
        self.params.reserve(params.len());
        self.param_values.reserve(params.len());

        for (name, value) in params {
            self.param_values.push(value.clone());
            self.params.insert(name, value);
        }
    }

    pub fn state<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.state.get::<T>()
    }

    #[cfg(feature = "database")]
    pub fn database(&self) -> crate::Result<&framework_database::Database> {
        self.state::<framework_database::Database>().ok_or_else(|| {
            crate::ConfigError::new("database", "database state is not configured").into()
        })
    }

    #[cfg(feature = "database")]
    pub fn connection(&self) -> crate::Result<RequestConnection<'_>> {
        let mut connection = self.connection.try_borrow_mut().map_err(|_| {
            framework_database::DatabaseError::new(
                framework_database::ErrorKind::Connection,
                "request database connection is already borrowed",
            )
        })?;
        if connection.is_none() {
            *connection = Some(self.database()?.acquire()?);
        }
        Ok(RequestConnection { inner: connection })
    }

    #[cfg(feature = "database")]
    pub fn transaction<T>(
        &self,
        options: framework_database::TransactionOptions,
        operation: impl FnOnce(&mut dyn framework_database::Connection) -> crate::Result<T>,
    ) -> crate::Result<T> {
        let mut connection = self.connection()?;
        let driver = connection.driver();
        let capabilities = connection.capabilities();
        let mut transaction = connection.begin(options)?;
        let result = {
            let mut transactional = RequestTransactionConnection {
                transaction: &mut *transaction,
                driver,
                capabilities,
            };
            operation(&mut transactional)
        };

        match result {
            Ok(value) => {
                transaction.commit()?;
                Ok(value)
            }
            Err(error) => {
                transaction.rollback()?;
                Err(error)
            }
        }
    }

    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub(crate) fn set_request_id(&mut self, id: String) {
        self.request_id = Some(id);
    }

    pub fn trace_context(&self) -> Option<&crate::operational::TraceContext> {
        self.trace_context.as_ref()
    }

    pub(crate) fn set_trace_context(&mut self, context: crate::operational::TraceContext) {
        self.trace_context = Some(context);
    }

    #[cfg(feature = "auth")]
    pub fn principal(&self) -> Option<&framework_auth::Principal> {
        self.principal.as_ref()
    }

    #[cfg(feature = "auth")]
    pub(crate) fn set_principal(&mut self, principal: framework_auth::Principal) {
        self.principal = Some(principal);
    }

    pub(crate) fn set_state(&mut self, state: crate::state::StateMap) {
        self.state = state;
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &str {
        self.target
            .split_once('?')
            .map_or(self.target.as_str(), |(path, _)| path)
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(String::as_str)
    }

    /// Parse a named route parameter without allocating an intermediate value.
    pub fn param_as<T: FromStr>(&self, name: &str) -> Option<Result<T, T::Err>> {
        self.param(name).map(str::parse)
    }

    pub(crate) fn param_at(&self, index: usize) -> Option<&str> {
        self.param_values.get(index).map(String::as_str)
    }

    pub(crate) fn param_at_as<T: FromStr>(&self, index: usize) -> Option<Result<T, T::Err>> {
        self.param_at(index).map(str::parse)
    }

    pub fn query_string(&self) -> Option<&str> {
        self.target.split_once('?').map(|(_, query)| query)
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn text(&self) -> crate::Result<&str> {
        std::str::from_utf8(&self.body).map_err(|error| HttpError::InvalidUtf8(error).into())
    }
}
