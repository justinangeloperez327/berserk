use crate::naming::{snake_case, validate_type_name};

/// Request source generation mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RequestKind {
    /// A standalone FormRequest scaffold.
    Basic,
    /// A FormRequest scaffold with explicit Claw create/update mappings.
    ModelBound { model: String },
}

/// Input for Berserk request source generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestSpec {
    name: String,
    kind: RequestKind,
}

impl RequestSpec {
    pub fn basic(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: RequestKind::Basic,
        }
    }

    pub fn model_bound(name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: RequestKind::ModelBound {
                model: model.into(),
            },
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> &RequestKind {
        &self.kind
    }
}

/// Generate one ordinary Rust FormRequest source file.
///
/// The initial scaffold intentionally contains one required string field named
/// `name`. Applications can edit or future codegen layers can replace this
/// field list; model-bound generation only adds explicit Claw write mappings
/// and never reflects over a model at runtime.
pub fn request_source(spec: &RequestSpec) -> syn::Result<String> {
    validate_type_name(spec.name(), "request")?;
    let writes = match spec.kind() {
        RequestKind::Basic => String::new(),
        RequestKind::ModelBound { model } => {
            validate_type_name(model, "model")?;
            model_writes(spec.name(), model)
        }
    };

    Ok(format!(
        "use berserk::{{FormRequest, FromJson, Json, ValidationErrors, ValidationResult}};\n\npub struct {name} {{\n    pub name: String,\n}}\n\nimpl FromJson for {name} {{\n    fn from_json(value: &Json) -> Result<Self, ValidationErrors> {{\n        let name = value.get(\"name\").and_then(Json::as_str);\n        let mut errors = ValidationErrors::default();\n        if name.is_none() {{\n            errors.add(\"name\", \"string\", \"A name string is required.\");\n        }}\n        errors.finish()?;\n        Ok(Self {{\n            name: name.unwrap_or_default().into(),\n        }})\n    }}\n}}\n\nimpl FormRequest for {name} {{\n    fn sanitize(&mut self) {{\n        self.name = self.name.trim().into();\n    }}\n\n    fn validate(&self) -> ValidationResult {{\n        let mut errors = ValidationErrors::default();\n        errors.length(\"name\", &self.name, 1, 100);\n        errors.finish()\n    }}\n}}\n{writes}",
        name = spec.name(),
    ))
}

fn model_writes(request: &str, model: &str) -> String {
    let model_module = snake_case(model);
    format!(
        "\nuse berserk::claw::{{IntoInsert, IntoUpdate, Result as ClawResult, Value}};\nuse crate::app::models::{model_module}::{model};\n\nimpl IntoInsert<{model}> for {request} {{\n    fn into_insert(self) -> ClawResult<Vec<(String, Value)>> {{\n        Ok(vec![(\"name\".into(), self.name.into())])\n    }}\n}}\n\nimpl IntoUpdate<{model}> for {request} {{\n    fn into_update(self) -> ClawResult<Vec<(String, Value)>> {{\n        Ok(vec![(\"name\".into(), self.name.into())])\n    }}\n}}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_request_is_valid_rust() {
        let source = request_source(&RequestSpec::basic("UserInput")).unwrap();
        syn::parse_file(&source).unwrap();
        assert!(source.contains("impl FormRequest for UserInput"));
        assert!(!source.contains("IntoInsert"));
    }

    #[test]
    fn model_bound_request_generates_explicit_writes() {
        let source = request_source(&RequestSpec::model_bound("UserInput", "User")).unwrap();
        syn::parse_file(&source).unwrap();
        assert!(source.contains("impl IntoInsert<User> for UserInput"));
        assert!(source.contains("impl IntoUpdate<User> for UserInput"));
        assert!(source.contains("crate::app::models::user::User"));
    }

    #[test]
    fn invalid_names_are_rejected() {
        assert!(request_source(&RequestSpec::basic("user_input")).is_err());
        assert!(request_source(&RequestSpec::model_bound("UserInput", "user")).is_err());
    }
}
