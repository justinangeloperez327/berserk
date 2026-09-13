use crate::{OpenApiError, Result};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    Object,
    Array,
    String,
    Integer,
    Number,
    Boolean,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Schema {
    #[serde(rename = "type")]
    kind: SchemaType,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    properties: BTreeMap<String, SchemaRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    required: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Box<SchemaRef>>,
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    enum_values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    example: Option<serde_json::Value>,
}

impl Schema {
    pub fn new(kind: SchemaType) -> Self {
        Self {
            kind,
            format: None,
            description: None,
            properties: BTreeMap::new(),
            required: Vec::new(),
            items: None,
            enum_values: Vec::new(),
            example: None,
        }
    }
    pub fn object() -> Self {
        Self::new(SchemaType::Object)
    }
    pub fn string() -> Self {
        Self::new(SchemaType::String)
    }
    pub fn integer() -> Self {
        Self::new(SchemaType::Integer)
    }
    pub fn boolean() -> Self {
        Self::new(SchemaType::Boolean)
    }
    pub fn array(items: SchemaRef) -> Self {
        let mut schema = Self::new(SchemaType::Array);
        schema.items = Some(Box::new(items));
        schema
    }
    pub fn format(mut self, value: impl Into<String>) -> Self {
        self.format = Some(value.into());
        self
    }
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }
    pub fn property(
        mut self,
        name: impl Into<String>,
        schema: impl Into<SchemaRef>,
    ) -> Result<Self> {
        let name = name.into();
        validate_name(&name, "property")?;
        if self.kind != SchemaType::Object {
            return Err(OpenApiError::new("properties require an object schema"));
        }
        if self.properties.contains_key(&name) {
            return Err(OpenApiError::new(format!("duplicate property `{name}`")));
        }
        self.properties.insert(name, schema.into());
        Ok(self)
    }
    pub fn required(mut self, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if !self.properties.contains_key(&name) {
            return Err(OpenApiError::new(format!(
                "required property `{name}` is not defined"
            )));
        }
        if self.required.contains(&name) {
            return Err(OpenApiError::new(format!(
                "required property `{name}` is duplicated"
            )));
        }
        self.required.push(name);
        Ok(self)
    }
    pub fn enum_values<I, S>(mut self, values: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        if self.kind != SchemaType::String {
            return Err(OpenApiError::new(
                "enum values currently require a string schema",
            ));
        }
        self.enum_values = values.into_iter().map(Into::into).collect();
        if self.enum_values.is_empty() {
            return Err(OpenApiError::new("enum values cannot be empty"));
        }
        for (index, value) in self.enum_values.iter().enumerate() {
            if self.enum_values[..index].contains(value) {
                return Err(OpenApiError::new(format!("duplicate enum value `{value}`")));
            }
        }
        Ok(self)
    }
    pub fn example(mut self, value: serde_json::Value) -> Self {
        self.example = Some(value);
        self
    }

    pub(crate) fn collect_references<'a>(&'a self, output: &mut Vec<&'a str>) {
        for schema in self.properties.values() {
            schema.collect_references(output);
        }
        if let Some(items) = &self.items {
            items.collect_references(output);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SchemaRef {
    Reference {
        #[serde(rename = "$ref")]
        reference: String,
    },
    Inline(Schema),
}

impl SchemaRef {
    pub fn named(name: &str) -> Result<Self> {
        validate_name(name, "schema")?;
        Ok(Self::Reference {
            reference: format!("#/components/schemas/{name}"),
        })
    }
    pub(crate) fn collect_references<'a>(&'a self, output: &mut Vec<&'a str>) {
        match self {
            Self::Reference { reference } => {
                if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                    output.push(name);
                }
            }
            Self::Inline(schema) => schema.collect_references(output),
        }
    }
}
impl From<Schema> for SchemaRef {
    fn from(schema: Schema) -> Self {
        Self::Inline(schema)
    }
}

pub(crate) fn validate_name(name: &str, kind: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(OpenApiError::new(format!(
            "{kind} name `{name}` is invalid"
        )));
    }
    Ok(())
}
