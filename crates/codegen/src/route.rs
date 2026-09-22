use crate::naming::{snake_case, validate_type_name};

const CONTROLLER_IMPORT_MARKER: &str = "// berserk:generated-controller-imports";
const ROUTE_MARKER: &str = "    // berserk:generated-routes";

/// One generated route registration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RouteSpec {
    /// A model-bound REST resource registered through `Route::crud`.
    Crud { path: String, controller: String },
}

impl RouteSpec {
    pub fn crud(path: impl Into<String>, controller: impl Into<String>) -> Self {
        Self::Crud {
            path: path.into(),
            controller: controller.into(),
        }
    }
}

/// Source-level specification for an application's route registration file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RoutesSpec {
    welcome: bool,
    health: bool,
    routes: Vec<RouteSpec>,
}

impl RoutesSpec {
    /// Create an empty route file.
    pub const fn new() -> Self {
        Self {
            welcome: false,
            health: false,
            routes: Vec::new(),
        }
    }

    /// Create the conventional route file used by `berserk new`.
    pub fn application() -> Self {
        Self::new().welcome().health()
    }

    pub fn welcome(mut self) -> Self {
        self.welcome = true;
        self
    }

    pub fn health(mut self) -> Self {
        self.health = true;
        self
    }

    pub fn route(mut self, route: RouteSpec) -> Self {
        self.routes.push(route);
        self
    }

    pub fn crud(self, path: impl Into<String>, controller: impl Into<String>) -> Self {
        self.route(RouteSpec::crud(path, controller))
    }

    pub const fn includes_welcome(&self) -> bool {
        self.welcome
    }

    pub const fn includes_health(&self) -> bool {
        self.health
    }

    pub fn routes(&self) -> &[RouteSpec] {
        &self.routes
    }
}

/// Generate one ordinary Rust application route-registration module.
///
/// Codegen emits explicit calls to Berserk's public route registrar. It does
/// not create a second runtime router or hide route registration behind global
/// state.
pub fn routes_source(spec: &RoutesSpec) -> syn::Result<String> {
    let mut controller_imports = Vec::<String>::new();

    for route in spec.routes() {
        match route {
            RouteSpec::Crud { path, controller } => {
                validate_crud_path(path)?;
                validate_type_name(controller, "controller")?;
                if !controller_imports.iter().any(|name| name == controller) {
                    controller_imports.push(controller.clone());
                }
            }
        }
    }

    let mut source = String::new();

    for controller in &controller_imports {
        source.push_str(&format!(
            "use crate::app::controllers::{}::{};\n",
            snake_case(controller),
            controller
        ));
    }
    source.push_str(CONTROLLER_IMPORT_MARKER);
    source.push('\n');
    if spec.includes_welcome() {
        source.push_str("use crate::config::AppConfig;\n");
    }

    let mut berserk_imports = vec!["App", "Result"];
    if spec.includes_welcome() || spec.includes_health() {
        berserk_imports.insert(0, "response");
    }
    if spec.includes_welcome() {
        let insert_at = berserk_imports.len() - 1;
        berserk_imports.insert(insert_at, "Request");
    }
    source.push_str(&format!(
        "use berserk::{{{}}};\n\n",
        berserk_imports.join(", ")
    ));

    source.push_str("pub fn register(app: &mut App) -> Result<()> {\n");

    if spec.includes_welcome() {
        source.push_str(
            "    app.route().get(\"/\", |request: Request| {\n        response().text(request.config::<AppConfig>()?.name.clone())\n    })?;\n",
        );
    }
    if spec.includes_health() {
        source.push_str("    app.route().get(\"/health\", || response().text(\"OK\"))?;\n");
    }

    for route in spec.routes() {
        match route {
            RouteSpec::Crud { path, controller } => {
                source.push_str(&format!(
                    "    app.route().crud(\"{path}\", {controller})?;\n"
                ));
            }
        }
    }
    source.push_str(ROUTE_MARKER);
    source.push('\n');

    source.push_str("    Ok(())\n}\n");
    Ok(source)
}

pub fn register_route_source(source: &str, route: &RouteSpec) -> syn::Result<String> {
    syn::parse_file(source)?;
    if source.matches(CONTROLLER_IMPORT_MARKER).count() != 1
        || source.matches(ROUTE_MARKER).count() != 1
    {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "route file is missing the Berserk generated-route markers",
        ));
    }

    match route {
        RouteSpec::Crud { path, controller } => {
            validate_crud_path(path)?;
            validate_type_name(controller, "controller")?;
            let import = format!(
                "use crate::app::controllers::{}::{};",
                snake_case(controller),
                controller
            );
            let registration = format!("    app.route().crud(\"{path}\", {controller})?;");

            if source.lines().any(|line| line.trim() == import)
                || source
                    .lines()
                    .any(|line| line.trim() == registration.trim())
            {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "route or controller import is already registered",
                ));
            }

            let source = source.replacen(
                CONTROLLER_IMPORT_MARKER,
                &format!("{import}\n{CONTROLLER_IMPORT_MARKER}"),
                1,
            );
            Ok(source.replacen(ROUTE_MARKER, &format!("{registration}\n{ROUTE_MARKER}"), 1))
        }
    }
}

fn validate_crud_path(path: &str) -> syn::Result<()> {
    let valid = path.len() > 1
        && path.starts_with('/')
        && !path.ends_with('/')
        && !path.contains(['?', '#', '*', '{', '}'])
        && path.split('/').skip(1).all(|segment| {
            !segment.is_empty()
                && segment.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~')
                })
        });

    if valid {
        Ok(())
    } else {
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "CRUD route path must be an absolute static path without parameters or a trailing slash",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_routes_match_the_generated_project_contract() {
        let source = routes_source(&RoutesSpec::application()).unwrap();
        syn::parse_file(&source).unwrap();
        assert!(source.contains("request.config::<AppConfig>()"));
        assert!(source.contains("app.route().get(\"/health\""));
    }

    #[test]
    fn crud_routes_import_and_register_the_controller() {
        let source = routes_source(&RoutesSpec::new().crud("/users", "UserController")).unwrap();

        syn::parse_file(&source).unwrap();
        assert!(source.contains("use crate::app::controllers::user_controller::UserController;"));
        assert!(source.contains("app.route().crud(\"/users\", UserController)?;"));
    }

    #[test]
    fn duplicate_controller_imports_are_collapsed() {
        let source = routes_source(
            &RoutesSpec::new()
                .crud("/users", "UserController")
                .crud("/admins", "UserController"),
        )
        .unwrap();

        assert_eq!(
            source
                .matches("use crate::app::controllers::user_controller::UserController;")
                .count(),
            1
        );
    }

    #[test]
    fn generated_route_can_be_registered_without_replacing_the_file() {
        let source = routes_source(&RoutesSpec::application()).unwrap();
        let updated =
            register_route_source(&source, &RouteSpec::crud("/users", "UserController")).unwrap();

        syn::parse_file(&updated).unwrap();
        assert!(updated.contains("use crate::app::controllers::user_controller::UserController;"));
        assert!(updated.contains("app.route().crud(\"/users\", UserController)?;"));
        assert!(
            register_route_source(&updated, &RouteSpec::crud("/users", "UserController")).is_err()
        );
    }

    #[test]
    fn invalid_crud_routes_are_rejected() {
        assert!(routes_source(&RoutesSpec::new().crud("users", "UserController")).is_err());
        assert!(routes_source(&RoutesSpec::new().crud("/users/{id}", "UserController")).is_err());
        assert!(routes_source(&RoutesSpec::new().crud("/users/", "UserController")).is_err());
    }
}
