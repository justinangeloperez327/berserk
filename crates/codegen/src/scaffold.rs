use crate::naming::validate_type_name;

/// Input for a generated middleware scaffold.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiddlewareSpec {
    name: String,
}

impl MiddlewareSpec {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Input for a generated API-resource scaffold.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceSpec {
    name: String,
    id_type: String,
}

impl ResourceSpec {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id_type: "i64".into(),
        }
    }

    pub fn id_type(mut self, rust_type: impl Into<String>) -> Self {
        self.id_type = rust_type.into();
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn identifier_type(&self) -> &str {
        &self.id_type
    }
}

/// Input for a generated authorization-policy scaffold.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicySpec {
    name: String,
    resource_type: String,
}

impl PolicySpec {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            resource_type: "u64".into(),
        }
    }

    pub fn resource_type(mut self, rust_type: impl Into<String>) -> Self {
        self.resource_type = rust_type.into();
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn policy_resource_type(&self) -> &str {
        &self.resource_type
    }
}

pub fn middleware_source(spec: &MiddlewareSpec) -> syn::Result<String> {
    validate_type_name(spec.name(), "middleware")?;
    Ok(format!(
        "use berserk::{{Middleware, Next, Request, Response, Result}};\n\npub struct {name};\n\nimpl Middleware for {name} {{\n    fn handle(&self, request: Request, next: Next<'_>) -> Result<Response> {{\n        next.run(request)\n    }}\n}}\n",
        name = spec.name(),
    ))
}

pub fn resource_source(spec: &ResourceSpec) -> syn::Result<String> {
    validate_type_name(spec.name(), "resource")?;
    validate_rust_type(spec.identifier_type(), "resource identifier")?;
    Ok(format!(
        "use berserk::{{ApiResource, Json}};\n\npub struct {name} {{\n    pub id: {id_type},\n}}\n\nimpl ApiResource for {name} {{\n    fn to_resource(&self) -> Json {{\n        Json::Object([(\"id\".into(), self.id.into())].into())\n    }}\n}}\n",
        name = spec.name(),
        id_type = spec.identifier_type(),
    ))
}

pub fn policy_source(spec: &PolicySpec) -> syn::Result<String> {
    validate_type_name(spec.name(), "policy")?;
    validate_rust_type(spec.policy_resource_type(), "policy resource")?;
    Ok(format!(
        "use berserk::auth::{{Ability, Decision, Policy, Principal}};\n\npub struct {name};\n\nimpl Policy<{resource_type}> for {name} {{\n    fn authorize(\n        &self,\n        _principal: &Principal,\n        _ability: &Ability,\n        _resource: &{resource_type},\n    ) -> Decision {{\n        Decision::Deny\n    }}\n}}\n",
        name = spec.name(),
        resource_type = spec.policy_resource_type(),
    ))
}

fn validate_rust_type(value: &str, kind: &str) -> syn::Result<()> {
    syn::parse_str::<syn::Type>(value).map(|_| ()).map_err(|_| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("{kind} must be a valid Rust type"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middleware_scaffold_is_valid_rust() {
        let source = middleware_source(&MiddlewareSpec::new("Audit")).unwrap();
        syn::parse_file(&source).unwrap();
        assert!(source.contains("impl Middleware for Audit"));
    }

    #[test]
    fn resource_scaffold_is_valid_rust() {
        let source = resource_source(&ResourceSpec::new("UserResource")).unwrap();
        syn::parse_file(&source).unwrap();
        assert!(source.contains("pub id: i64"));
        assert!(source.contains("impl ApiResource for UserResource"));
    }

    #[test]
    fn resource_identifier_type_can_be_customized() {
        let source = resource_source(&ResourceSpec::new("UserResource").id_type("String")).unwrap();
        assert!(source.contains("pub id: String"));
    }

    #[test]
    fn policy_scaffold_denies_by_default() {
        let source = policy_source(&PolicySpec::new("UserPolicy")).unwrap();
        syn::parse_file(&source).unwrap();
        assert!(source.contains("impl Policy<u64> for UserPolicy"));
        assert!(source.contains("Decision::Deny"));
    }

    #[test]
    fn scaffold_names_and_types_are_validated() {
        assert!(middleware_source(&MiddlewareSpec::new("audit")).is_err());
        assert!(resource_source(&ResourceSpec::new("UserResource").id_type("not a type")).is_err());
        assert!(policy_source(&PolicySpec::new("UserPolicy").resource_type("not a type")).is_err());
    }
}
