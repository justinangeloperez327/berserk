use crate::naming::{snake_case, validate_type_name};
use crate::{
    controller_source, migration_source, model_source, request_source, ControllerSpec, FieldSpec,
    MigrationColumnSpec, MigrationSpec, ModelSpec, RequestSpec, RouteSpec,
};

/// One generated file belonging to a coordinated application module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleFile {
    path: String,
    content: String,
}

impl ModuleFile {
    fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn into_parts(self) -> (String, String) {
        (self.path, self.content)
    }
}

/// Shared source-level specification for one conventional Berserk CRUD module.
///
/// A module owns one coherent set of names so its model, request, controller,
/// create-table migration, and route cannot silently drift apart.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleSpec {
    name: String,
    model: ModelSpec,
    request: RequestSpec,
    controller: ControllerSpec,
    migration: MigrationSpec,
    route: RouteSpec,
}

impl ModuleSpec {
    /// Build a conventional CRUD module.
    ///
    /// The initial scaffold contains an `id` primary key plus one fillable
    /// `name: String` field so the generated request, Claw writes, model, and
    /// create-table migration describe the same minimal resource.
    pub fn crud(name: impl Into<String>, timestamp: u64) -> syn::Result<Self> {
        let name = name.into();
        validate_type_name(&name, "module")?;

        let model = ModelSpec::new(&name).field(FieldSpec::string("name").fillable());
        let model_name = model.name().to_owned();
        let table = model.table_name();
        let request_name = format!("{name}Input");
        let controller_name = format!("{name}Controller");
        let migration_name = format!("create_{table}_table");

        Ok(Self {
            name,
            model,
            request: RequestSpec::model_bound(&request_name, &model_name),
            controller: ControllerSpec::crud(&controller_name, &model_name, &request_name),
            migration: MigrationSpec::new(migration_name, timestamp)
                .column(MigrationColumnSpec::string("name")),
            route: RouteSpec::crud(format!("/{table}"), controller_name),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn model(&self) -> &ModelSpec {
        &self.model
    }

    pub fn request(&self) -> &RequestSpec {
        &self.request
    }

    pub fn controller(&self) -> &ControllerSpec {
        &self.controller
    }

    pub fn migration(&self) -> &MigrationSpec {
        &self.migration
    }

    pub fn route(&self) -> &RouteSpec {
        &self.route
    }
}

/// Generate the Rust source files for one coordinated module.
///
/// Route registration is returned by `ModuleSpec::route()` because an existing
/// application's route file must be updated rather than replaced.
pub fn module_files(spec: &ModuleSpec) -> syn::Result<Vec<ModuleFile>> {
    validate_type_name(spec.name(), "module")?;

    let model = model_source(spec.model())?;
    let request = request_source(spec.request())?;
    let controller = controller_source(spec.controller())?;
    let migration = migration_source(spec.migration())?;

    Ok(vec![
        ModuleFile::new(
            format!("src/app/models/{}.rs", snake_case(spec.model().name())),
            model,
        ),
        ModuleFile::new(
            format!(
                "src/app/validations/{}.rs",
                snake_case(spec.request().name())
            ),
            request,
        ),
        ModuleFile::new(
            format!(
                "src/app/controllers/{}.rs",
                snake_case(spec.controller().name())
            ),
            controller,
        ),
        ModuleFile::new(
            format!(
                "src/database/migrations/{}_{}.rs",
                spec.migration().timestamp(),
                spec.migration().name()
            ),
            migration,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crud_module_coordinates_all_names() {
        let module = ModuleSpec::crud("User", 1_789_994_000).unwrap();

        assert_eq!(module.model().name(), "User");
        assert_eq!(module.model().table_name(), "users");
        assert_eq!(module.request().name(), "UserInput");
        assert_eq!(module.controller().name(), "UserController");
        assert_eq!(module.migration().name(), "create_users_table");
        assert_eq!(module.migration().timestamp(), 1_789_994_000);
        assert_eq!(module.route(), &RouteSpec::crud("/users", "UserController"));
    }

    #[test]
    fn crud_module_sources_are_valid_rust() {
        let module = ModuleSpec::crud("User", 1_789_994_000).unwrap();
        let files = module_files(&module).unwrap();

        assert_eq!(files.len(), 4);
        for file in &files {
            syn::parse_file(file.content()).unwrap();
        }

        let model = files
            .iter()
            .find(|file| file.path() == "src/app/models/user.rs")
            .unwrap();
        assert!(model.content().contains("#[fillable]"));
        assert!(model.content().contains("pub name: String"));

        let migration = files
            .iter()
            .find(|file| file.path().contains("create_users_table.rs"))
            .unwrap();
        assert!(migration.content().contains("Column::string(\"name\")"));
    }

    #[test]
    fn invalid_module_name_is_rejected() {
        assert!(ModuleSpec::crud("user", 1).is_err());
    }
}
