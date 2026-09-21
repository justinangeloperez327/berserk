use crate::naming::{snake_case, validate_type_name};

/// Controller source generation mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControllerKind {
    /// A small controller with index/show placeholders.
    Basic,
    /// A Claw model-bound CRUD controller for `Route::crud`.
    Crud { model: String, request: String },
}

/// Input for Berserk controller source generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSpec {
    name: String,
    kind: ControllerKind,
}

impl ControllerSpec {
    pub fn basic(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: ControllerKind::Basic,
        }
    }

    pub fn crud(
        name: impl Into<String>,
        model: impl Into<String>,
        request: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: ControllerKind::Crud {
                model: model.into(),
                request: request.into(),
            },
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> &ControllerKind {
        &self.kind
    }
}

/// Generate one ordinary Rust controller source file.
///
/// This function generates source only. Filesystem placement and module
/// registration remain responsibilities of the CLI or another build tool.
pub fn controller_source(spec: &ControllerSpec) -> syn::Result<String> {
    validate_type_name(spec.name(), "controller")?;
    match spec.kind() {
        ControllerKind::Basic => Ok(basic_controller(spec.name())),
        ControllerKind::Crud { model, request } => {
            validate_type_name(model, "model")?;
            validate_type_name(request, "request")?;
            Ok(crud_controller(spec.name(), model, request))
        }
    }
}

fn basic_controller(name: &str) -> String {
    format!(
        "use berserk::{{response, Json, Response, Result}};\n\npub struct {name};\n\nimpl {name} {{\n    pub fn index() -> Result<Response> {{\n        response().json(Json::Array(vec![]))\n    }}\n\n    pub fn show(id: u64) -> Result<Response> {{\n        response().json(Json::Object([(\"id\".into(), id.into())].into()))\n    }}\n}}\n"
    )
}

fn crud_controller(name: &str, model: &str, request: &str) -> String {
    let model_module = snake_case(model);
    let request_module = snake_case(request);
    format!(
        "use berserk::{{response, CrudController, Model, Response, Result}};\n\nuse crate::app::models::{model_module}::{model};\nuse crate::app::validations::{request_module}::{request};\n\npub struct {name};\n\nimpl CrudController for {name} {{\n    type Model = {model};\n    type Create = {request};\n    type Update = {request};\n\n    fn index(&self) -> Result<Response> {{\n        response().json({model}::all()?)\n    }}\n\n    fn store(&self, input: Self::Create) -> Result<Response> {{\n        let model = {model}::create(input)?;\n        response().status(201).json(model)\n    }}\n\n    fn show(&self, model: Self::Model) -> Result<Response> {{\n        response().json(model)\n    }}\n\n    fn update(&self, mut model: Self::Model, input: Self::Update) -> Result<Response> {{\n        model.update(input)?;\n        response().json(model)\n    }}\n\n    fn destroy(&self, model: Self::Model) -> Result<Response> {{\n        model.delete()?;\n        response().no_content()\n    }}\n}}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_controller_is_valid_rust() {
        let source = controller_source(&ControllerSpec::basic("UserController")).unwrap();
        syn::parse_file(&source).unwrap();
        assert!(source.contains("pub struct UserController;"));
    }

    #[test]
    fn crud_controller_uses_model_binding_contract() {
        let source =
            controller_source(&ControllerSpec::crud("UserController", "User", "UserInput"))
                .unwrap();
        syn::parse_file(&source).unwrap();
        assert!(source.contains("impl CrudController for UserController"));
        assert!(source.contains("type Model = User;"));
        assert!(source.contains("type Create = UserInput;"));
        assert!(source.contains("response().status(201).json(model)"));
    }

    #[test]
    fn invalid_type_names_are_rejected() {
        assert!(controller_source(&ControllerSpec::basic("user_controller")).is_err());
        assert!(
            controller_source(&ControllerSpec::crud("UserController", "user", "UserInput"))
                .is_err()
        );
    }
}
