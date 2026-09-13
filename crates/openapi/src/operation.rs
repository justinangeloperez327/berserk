use crate::{schema::validate_name, OpenApiError, Result, SchemaRef};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Parameter {
    name: String,
    #[serde(rename = "in")]
    location: ParameterLocation,
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    schema: SchemaRef,
}

impl Parameter {
    pub fn new(
        name: impl Into<String>,
        location: ParameterLocation,
        schema: impl Into<SchemaRef>,
    ) -> Result<Self> {
        let name = name.into();
        validate_name(&name, "parameter")?;
        Ok(Self {
            name,
            location,
            required: location == ParameterLocation::Path,
            description: None,
            schema: schema.into(),
        })
    }
    pub fn required(mut self, value: bool) -> Result<Self> {
        if self.location == ParameterLocation::Path && !value {
            return Err(OpenApiError::new("path parameters must be required"));
        }
        self.required = value;
        Ok(self)
    }
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub const fn location(&self) -> ParameterLocation {
        self.location
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RequestBody {
    required: bool,
    content: BTreeMap<String, MediaType>,
}

impl RequestBody {
    pub fn json(schema: impl Into<SchemaRef>) -> Self {
        Self {
            required: true,
            content: BTreeMap::from([(
                "application/json".into(),
                MediaType {
                    schema: schema.into(),
                },
            )]),
        }
    }
    pub fn required(mut self, value: bool) -> Self {
        self.required = value;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct MediaType {
    schema: SchemaRef,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ApiResponse {
    description: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    content: BTreeMap<String, MediaType>,
}

impl ApiResponse {
    pub fn new(description: impl Into<String>) -> Result<Self> {
        let description = description.into();
        if description.trim().is_empty() {
            return Err(OpenApiError::new("response description cannot be empty"));
        }
        Ok(Self {
            description,
            content: BTreeMap::new(),
        })
    }
    pub fn json(mut self, schema: impl Into<SchemaRef>) -> Self {
        self.content.insert(
            "application/json".into(),
            MediaType {
                schema: schema.into(),
            },
        );
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    operation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    parameters: Vec<Parameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_body: Option<RequestBody>,
    responses: BTreeMap<String, ApiResponse>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    security: Vec<BTreeMap<String, Vec<String>>>,
}

impl Operation {
    pub fn new(operation_id: impl Into<String>) -> Result<Self> {
        let operation_id = operation_id.into();
        validate_name(&operation_id, "operation")?;
        Ok(Self {
            operation_id,
            summary: None,
            description: None,
            tags: Vec::new(),
            parameters: Vec::new(),
            request_body: None,
            responses: BTreeMap::new(),
            security: Vec::new(),
        })
    }
    pub fn summary(mut self, value: impl Into<String>) -> Self {
        self.summary = Some(value.into());
        self
    }
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }
    pub fn tag(mut self, value: impl Into<String>) -> Self {
        self.tags.push(value.into());
        self
    }
    pub fn parameter(mut self, parameter: Parameter) -> Result<Self> {
        if self
            .parameters
            .iter()
            .any(|item| item.name == parameter.name && item.location == parameter.location)
        {
            return Err(OpenApiError::new(format!(
                "duplicate parameter `{}`",
                parameter.name
            )));
        }
        self.parameters.push(parameter);
        Ok(self)
    }
    pub fn request_body(mut self, body: RequestBody) -> Result<Self> {
        if self.request_body.is_some() {
            return Err(OpenApiError::new("request body is already defined"));
        }
        self.request_body = Some(body);
        Ok(self)
    }
    pub fn response(mut self, status: impl Into<String>, response: ApiResponse) -> Result<Self> {
        let status = status.into();
        validate_status(&status)?;
        if self.responses.contains_key(&status) {
            return Err(OpenApiError::new(format!("duplicate response `{status}`")));
        }
        self.responses.insert(status, response);
        Ok(self)
    }
    pub fn secured_by<I, S>(mut self, scheme: impl Into<String>, scopes: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let scheme = scheme.into();
        validate_name(&scheme, "security scheme")?;
        self.security.push(BTreeMap::from([(
            scheme,
            scopes.into_iter().map(Into::into).collect(),
        )]));
        Ok(self)
    }
    pub(crate) fn validate_for_path(&self, path: &str) -> Result<()> {
        if self.responses.is_empty() {
            return Err(OpenApiError::new(format!(
                "operation `{}` requires at least one response",
                self.operation_id
            )));
        }
        let templates = template_names(path)?;
        let declared: Vec<_> = self
            .parameters
            .iter()
            .filter(|parameter| parameter.location == ParameterLocation::Path)
            .map(|parameter| parameter.name.as_str())
            .collect();
        if templates != declared {
            return Err(OpenApiError::new(format!(
                "path parameters for `{path}` do not match its template in declaration order"
            )));
        }
        Ok(())
    }
    pub(crate) fn collect_references<'a>(
        &'a self,
        schemas: &mut Vec<&'a str>,
        security: &mut Vec<&'a str>,
    ) {
        for parameter in &self.parameters {
            parameter.schema.collect_references(schemas);
        }
        if let Some(body) = &self.request_body {
            for media in body.content.values() {
                media.schema.collect_references(schemas);
            }
        }
        for response in self.responses.values() {
            for media in response.content.values() {
                media.schema.collect_references(schemas);
            }
        }
        for requirement in &self.security {
            security.extend(requirement.keys().map(String::as_str));
        }
    }
}

fn validate_status(status: &str) -> Result<()> {
    if status == "default" {
        return Ok(());
    }
    let valid = status.len() == 3
        && status.bytes().all(|byte| byte.is_ascii_digit())
        && status
            .parse::<u16>()
            .is_ok_and(|code| (100..=599).contains(&code));
    if valid {
        Ok(())
    } else {
        Err(OpenApiError::new(format!(
            "response status `{status}` is invalid"
        )))
    }
}

fn template_names(path: &str) -> Result<Vec<&str>> {
    if !path.starts_with('/') {
        return Err(OpenApiError::new("OpenAPI paths must start with `/`"));
    }
    let mut names = Vec::new();
    for segment in path.split('/') {
        if segment.contains('{') || segment.contains('}') {
            if !(segment.starts_with('{') && segment.ends_with('}') && segment.len() > 2) {
                return Err(OpenApiError::new(format!("invalid path template `{path}`")));
            }
            let name = &segment[1..segment.len() - 1];
            validate_name(name, "path parameter")?;
            if names.contains(&name) {
                return Err(OpenApiError::new(format!(
                    "duplicate path parameter `{name}`"
                )));
            }
            names.push(name);
        }
    }
    Ok(names)
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SecurityScheme {
    Http {
        scheme: String,
        #[serde(rename = "bearerFormat", skip_serializing_if = "Option::is_none")]
        bearer_format: Option<String>,
    },
    ApiKey {
        name: String,
        #[serde(rename = "in")]
        location: ParameterLocation,
    },
}

impl SecurityScheme {
    pub fn bearer(format: Option<impl Into<String>>) -> Self {
        Self::Http {
            scheme: "bearer".into(),
            bearer_format: format.map(Into::into),
        }
    }
    pub fn api_key(name: impl Into<String>, location: ParameterLocation) -> Result<Self> {
        if location == ParameterLocation::Path {
            return Err(OpenApiError::new("API keys cannot use path parameters"));
        }
        let name = name.into();
        validate_name(&name, "API key")?;
        Ok(Self::ApiKey { name, location })
    }
}
