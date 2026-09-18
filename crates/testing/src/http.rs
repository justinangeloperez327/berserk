use berserk::{App, Headers, Json, Method, Request, Response, Result};

pub struct TestClient<'a> {
    app: &'a App,
}
impl<'a> TestClient<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
    pub fn request(&self, method: &str, target: impl Into<String>) -> Result<TestRequest<'a>> {
        Ok(TestRequest {
            app: self.app,
            method: Method::new(method)?,
            target: target.into(),
            headers: Headers::new(),
            body: Vec::new(),
        })
    }
    pub fn get(&self, target: impl Into<String>) -> Result<TestResponse> {
        self.request("GET", target)?.send()
    }
    pub fn put(&self, target: impl Into<String>) -> Result<TestRequest<'a>> {
        self.request("PUT", target)
    }
    pub fn patch(&self, target: impl Into<String>) -> Result<TestRequest<'a>> {
        self.request("PATCH", target)
    }
    pub fn delete(&self, target: impl Into<String>) -> Result<TestRequest<'a>> {
        self.request("DELETE", target)
    }
    pub fn head(&self, target: impl Into<String>) -> Result<TestRequest<'a>> {
        self.request("HEAD", target)
    }
    pub fn post(&self, target: impl Into<String>) -> Result<TestRequest<'a>> {
        self.request("POST", target)
    }
}

pub struct TestRequest<'a> {
    app: &'a App,
    method: Method,
    target: String,
    headers: Headers,
    body: Vec<u8>,
}
impl<'a> TestRequest<'a> {
    pub fn header(mut self, name: &str, value: &str) -> Result<Self> {
        self.headers.append(name, value)?;
        Ok(self)
    }
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
    pub fn json(mut self, value: &Json) -> Result<Self> {
        self.headers.insert("content-type", "application/json")?;
        self.body = Response::json(value)?.body().to_vec();
        Ok(self)
    }
    pub fn send(self) -> Result<TestResponse> {
        Ok(TestResponse(self.app.respond(Request::new(
            self.method,
            self.target,
            self.headers,
            self.body,
        )?)))
    }
}

pub struct TestResponse(Response);
impl TestResponse {
    pub fn response(&self) -> &Response {
        &self.0
    }
    pub fn into_response(self) -> Response {
        self.0
    }
    #[track_caller]
    pub fn assert_status(self, expected: u16) -> Self {
        assert_eq!(self.0.status_code(), expected, "unexpected HTTP status");
        self
    }
    #[track_caller]
    pub fn assert_ok(self) -> Self {
        assert!(
            (200..300).contains(&self.0.status_code()),
            "expected successful HTTP status, got {}",
            self.0.status_code()
        );
        self
    }
    #[track_caller]
    pub fn assert_header(self, name: &str, expected: &str) -> Self {
        assert_eq!(
            self.0.headers().get(name),
            Some(expected),
            "unexpected `{name}` header"
        );
        self
    }
    #[track_caller]
    pub fn assert_header_missing(self, name: &str) -> Self {
        assert!(
            self.0.headers().get(name).is_none(),
            "unexpected `{name}` header"
        );
        self
    }
    #[track_caller]
    pub fn assert_body(self, expected: impl AsRef<[u8]>) -> Self {
        assert_eq!(self.0.body(), expected.as_ref(), "unexpected response body");
        self
    }
    #[track_caller]
    pub fn assert_text(self, expected: &str) -> Self {
        assert_eq!(
            std::str::from_utf8(self.0.body()).expect("response body is not UTF-8"),
            expected,
            "unexpected response text"
        );
        self
    }
    #[track_caller]
    pub fn assert_json(self, expected: &Json) -> Self {
        assert_eq!(
            &Json::parse(self.0.body()).expect("response body is not valid JSON"),
            expected,
            "unexpected JSON response"
        );
        self
    }
}

impl TestResponse {
    pub fn json(&self) -> Json {
        Json::parse(self.0.body()).expect("response body is not valid JSON")
    }
    #[track_caller]
    pub fn assert_json_path(self, path: &str, expected: impl Into<Json>) -> Self {
        assert_eq!(
            berserk::Arr::get(&self.json(), path),
            Some(&expected.into()),
            "unexpected JSON path value"
        );
        self
    }
    #[track_caller]
    pub fn assert_validation_error(self, field: &str) -> Self {
        assert_eq!(self.0.status_code(), 422);
        let json = self.json();
        let errors = json
            .get("errors")
            .and_then(|v| {
                if let Json::Array(items) = v {
                    Some(items)
                } else {
                    None
                }
            })
            .expect("validation errors array");
        assert!(
            errors
                .iter()
                .any(|e| e.get("field").and_then(Json::as_str) == Some(field)),
            "missing validation error for {field}"
        );
        self
    }
    pub fn assert_created(self) -> Self {
        self.assert_status(201)
    }
    pub fn assert_no_content(self) -> Self {
        self.assert_status(204)
    }
    pub fn assert_unauthorized(self) -> Self {
        self.assert_status(401)
    }
    pub fn assert_forbidden(self) -> Self {
        self.assert_status(403)
    }
    pub fn assert_not_found(self) -> Self {
        self.assert_status(404)
    }
}
