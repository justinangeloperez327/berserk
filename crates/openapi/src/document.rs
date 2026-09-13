use crate::{
    schema::validate_name, HttpMethod, OpenApiError, Operation, Result, Schema, SecurityScheme,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Info {
    title: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl Info {
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Result<Self> {
        let title = title.into();
        let version = version.into();
        if title.trim().is_empty() || version.trim().is_empty() {
            return Err(OpenApiError::new("API title and version cannot be empty"));
        }
        Ok(Self {
            title,
            version,
            description: None,
        })
    }
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    patch: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delete: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    head: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Operation>,
}

impl PathItem {
    fn slot(&mut self, method: HttpMethod) -> &mut Option<Operation> {
        match method {
            HttpMethod::Get => &mut self.get,
            HttpMethod::Post => &mut self.post,
            HttpMethod::Put => &mut self.put,
            HttpMethod::Patch => &mut self.patch,
            HttpMethod::Delete => &mut self.delete,
            HttpMethod::Head => &mut self.head,
            HttpMethod::Options => &mut self.options,
        }
    }
    fn operations(&self) -> impl Iterator<Item = &Operation> {
        [
            &self.get,
            &self.post,
            &self.put,
            &self.patch,
            &self.delete,
            &self.head,
            &self.options,
        ]
        .into_iter()
        .filter_map(Option::as_ref)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
struct Components {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    schemas: BTreeMap<String, Schema>,
    #[serde(
        rename = "securitySchemes",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    security_schemes: BTreeMap<String, SecurityScheme>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OpenApi {
    openapi: &'static str,
    info: Info,
    paths: BTreeMap<String, PathItem>,
    components: Components,
}

impl OpenApi {
    pub fn new(info: Info) -> Self {
        Self {
            openapi: "3.1.0",
            info,
            paths: BTreeMap::new(),
            components: Components::default(),
        }
    }
    pub fn schema(&mut self, name: impl Into<String>, schema: Schema) -> Result<()> {
        let name = name.into();
        validate_name(&name, "schema")?;
        if self.components.schemas.contains_key(&name) {
            return Err(OpenApiError::new(format!("duplicate schema `{name}`")));
        }
        self.components.schemas.insert(name, schema);
        Ok(())
    }
    pub fn security_scheme(
        &mut self,
        name: impl Into<String>,
        scheme: SecurityScheme,
    ) -> Result<()> {
        let name = name.into();
        validate_name(&name, "security scheme")?;
        if self.components.security_schemes.contains_key(&name) {
            return Err(OpenApiError::new(format!(
                "duplicate security scheme `{name}`"
            )));
        }
        self.components.security_schemes.insert(name, scheme);
        Ok(())
    }
    pub fn operation(
        &mut self,
        method: HttpMethod,
        path: impl Into<String>,
        operation: Operation,
    ) -> Result<()> {
        let path = path.into();
        operation.validate_for_path(&path)?;
        if self
            .paths
            .values()
            .flat_map(|item| item.operations())
            .any(|item| item.operation_id() == operation.operation_id())
        {
            return Err(OpenApiError::new(format!(
                "duplicate operation id `{}`",
                operation.operation_id()
            )));
        }
        let slot = self.paths.entry(path.clone()).or_default().slot(method);
        if slot.is_some() {
            return Err(OpenApiError::new(format!(
                "duplicate operation for `{path}`"
            )));
        }
        *slot = Some(operation);
        Ok(())
    }
    pub fn to_json(&self) -> Result<String> {
        let mut schemas = Vec::new();
        let mut security = Vec::new();
        for schema in self.components.schemas.values() {
            schema.collect_references(&mut schemas);
        }
        for operation in self.paths.values().flat_map(|path| path.operations()) {
            operation.collect_references(&mut schemas, &mut security);
        }
        for name in schemas {
            if !self.components.schemas.contains_key(name) {
                return Err(OpenApiError::new(format!(
                    "schema reference `{name}` is not registered"
                )));
            }
        }
        for name in security {
            if !self.components.security_schemes.contains_key(name) {
                return Err(OpenApiError::new(format!(
                    "security scheme `{name}` is not registered"
                )));
            }
        }
        serde_json::to_string_pretty(self)
            .map_err(|error| OpenApiError::new(format!("cannot encode OpenAPI document: {error}")))
    }
}
