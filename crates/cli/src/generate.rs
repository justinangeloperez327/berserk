use crate::{CliError, ErrorKind, Result};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedFile {
    pub path: PathBuf,
}
pub struct Generator {
    root: PathBuf,
}
impl Generator {
    pub fn at(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if !root.is_dir() {
            return Err(CliError::new(
                ErrorKind::UnsafePath,
                "generator root must be an existing directory",
            ));
        }
        Ok(Self {
            root: fs::canonicalize(root).map_err(CliError::from_io)?,
        })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn new_project(&self, relative: &Path) -> Result<Vec<GeneratedFile>> {
        validate_relative(relative)?;
        let target = self.root.join(relative);
        if target.exists() {
            return Err(CliError::new(
                ErrorKind::AlreadyExists,
                "project target already exists",
            ));
        }
        let package = relative
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                CliError::new(
                    ErrorKind::InvalidName,
                    "project path must end in a UTF-8 package name",
                )
            })?;
        validate_package(package)?;
        let parent = target
            .parent()
            .ok_or_else(|| CliError::new(ErrorKind::UnsafePath, "project path has no parent"))?;
        self.verify_existing_path(parent)?;
        if !parent.is_dir()
            || !fs::canonicalize(parent)
                .map_err(CliError::from_io)?
                .starts_with(&self.root)
        {
            return Err(CliError::new(
                ErrorKind::UnsafePath,
                "project parent must already exist inside the generator root",
            ));
        }
        fs::create_dir(&target).map_err(CliError::from_io)?;
        let result = (|| {
            for directory in [
                "src",
                "src/app",
                "src/app/controllers",
                "src/app/models",
                "src/app/validations",
                "src/app/middleware",
                "src/app/resources",
                "src/app/policies",
                "src/database",
                "src/database/migrations",
                "src/config",
            ] {
                fs::create_dir(target.join(directory)).map_err(CliError::from_io)?;
            }
            let version = env!("CARGO_PKG_VERSION");
            let rust_version = env!("CARGO_PKG_RUST_VERSION");
            let cargo = format!("[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"{rust_version}\"\n\n[dependencies]\nberserk = {{ version = \"{version}\", features = [\"claw\"] }}\n");
            write_new(&target.join("Cargo.toml"), cargo.as_bytes())?;
            let mut files = vec![GeneratedFile {
                path: target.join("Cargo.toml"),
            }];
            for (path, source) in [
                ("src/main.rs", include_str!("../templates/main.rs.stub")),
                (
                    "src/config/mod.rs",
                    include_str!("../templates/config.rs.stub"),
                ),
                (
                    "src/app/routes.rs",
                    include_str!("../templates/routes.rs.stub"),
                ),
            ] {
                let path = target.join(path);
                write_new(&path, source.as_bytes())?;
                files.push(GeneratedFile { path });
            }
            for (path, source) in [
                (
                    "src/app/mod.rs",
                    b"pub mod controllers;\npub mod middleware;\npub mod models;\npub mod policies;\npub mod resources;\npub mod routes;\npub mod validations;\n".as_slice(),
                ),
                ("src/app/controllers/mod.rs", b"".as_slice()),
                ("src/app/models/mod.rs", b"".as_slice()),
                ("src/app/validations/mod.rs", b"".as_slice()),
                ("src/app/middleware/mod.rs", b"".as_slice()),
                ("src/app/resources/mod.rs", b"".as_slice()),
                ("src/app/policies/mod.rs", b"".as_slice()),
                ("src/database/mod.rs", b"pub mod migrations;\n".as_slice()),
                ("src/database/migrations/mod.rs", b"".as_slice()),
            ] {
                let path = target.join(path);
                write_new(&path, source)?;
                files.push(GeneratedFile { path });
            }
            Ok(files)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&target);
        }
        result
    }
    pub fn make_model(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.make_type(
            name,
            "app/models",
            include_str!("../templates/model.rs.stub"),
        )
    }
    pub fn make_controller(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_type_name(name)?;
        let source =
            berserk_codegen::controller_source(&berserk_codegen::ControllerSpec::basic(name))
                .map_err(|error| CliError::new(ErrorKind::InvalidName, error.to_string()))?;
        self.make_source_type(name, "app/controllers", &source)
    }

    pub fn make_resource_controller(
        &self,
        name: &str,
        model: &str,
        request: &str,
    ) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_type_name(name)?;
        validate_type_name(model)?;
        validate_type_name(request)?;
        let source = berserk_codegen::controller_source(&berserk_codegen::ControllerSpec::crud(
            name, model, request,
        ))
        .map_err(|error| CliError::new(ErrorKind::InvalidName, error.to_string()))?;
        self.make_source_type(name, "app/controllers", &source)
    }
    pub fn make_request(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_type_name(name)?;
        let source = berserk_codegen::request_source(&berserk_codegen::RequestSpec::basic(name))
            .map_err(|error| CliError::new(ErrorKind::InvalidName, error.to_string()))?;
        self.make_source_type(name, "app/validations", &source)
    }

    pub fn make_model_request(&self, name: &str, model: &str) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_type_name(name)?;
        validate_type_name(model)?;
        let source = berserk_codegen::request_source(&berserk_codegen::RequestSpec::model_bound(
            name, model,
        ))
        .map_err(|error| CliError::new(ErrorKind::InvalidName, error.to_string()))?;
        self.make_source_type(name, "app/validations", &source)
    }
    pub fn make_middleware(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.make_type(
            name,
            "app/middleware",
            include_str!("../templates/middleware.rs.stub"),
        )
    }
    pub fn make_resource(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.make_type(
            name,
            "app/resources",
            include_str!("../templates/resource.rs.stub"),
        )
    }
    pub fn make_policy(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.make_type(
            name,
            "app/policies",
            include_str!("../templates/policy.rs.stub"),
        )
    }
    fn make_type(&self, name: &str, folder: &str, template: &str) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_type_name(name)?;
        let module = snake_case(name);
        let source = template
            .replace("{{name}}", name)
            .replace("{{table}}", &format!("{module}s"));
        self.make_source_type(name, folder, &source)
    }

    fn make_source_type(
        &self,
        name: &str,
        folder: &str,
        source: &str,
    ) -> Result<Vec<GeneratedFile>> {
        let module = snake_case(name);
        let directory = self.safe_directory(&format!("src/{folder}"))?;
        let source_path = directory.join(format!("{module}.rs"));
        let index = directory.join("mod.rs");
        write_new(&source_path, source.as_bytes())?;
        if let Err(error) = append_module(&index, &module) {
            let _ = fs::remove_file(&source_path);
            return Err(error);
        }
        Ok(vec![
            GeneratedFile { path: source_path },
            GeneratedFile { path: index },
        ])
    }
    pub fn make_migration(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_snake_name(name)?;
        let directory = self.safe_directory("src/database/migrations")?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CliError::new(ErrorKind::Clock, "system clock is before Unix epoch"))?
            .as_secs();
        let path = directory.join(format!("{timestamp}_{name}.rs"));
        let type_name = pascal_case(name);
        let source = migration_source(&type_name, timestamp, name);
        write_new(&path, source.as_bytes())?;
        Ok(vec![GeneratedFile { path }])
    }
    fn safe_directory(&self, relative: &str) -> Result<PathBuf> {
        let target = self.root.join(relative);
        let mut current = self.root.clone();
        for component in Path::new(relative).components() {
            let Component::Normal(component) = component else {
                return Err(CliError::new(
                    ErrorKind::UnsafePath,
                    "generator directory is not normalized",
                ));
            };
            current.push(component);
            match fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(CliError::new(
                        ErrorKind::UnsafePath,
                        "symbolic links are not allowed in generator paths",
                    ))
                }
                Ok(metadata) if !metadata.is_dir() => {
                    return Err(CliError::new(
                        ErrorKind::UnsafePath,
                        "generator path component is not a directory",
                    ))
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    fs::create_dir(&current).map_err(CliError::from_io)?
                }
                Err(error) => return Err(CliError::from_io(error)),
            }
        }
        let canonical = fs::canonicalize(&target).map_err(CliError::from_io)?;
        if !canonical.starts_with(&self.root) {
            return Err(CliError::new(
                ErrorKind::UnsafePath,
                "generator path escaped its root",
            ));
        }
        Ok(canonical)
    }
    fn ensure_application(&self) -> Result<()> {
        if self.root.join("Cargo.toml").is_file() {
            Ok(())
        } else {
            Err(CliError::new(
                ErrorKind::UnsafePath,
                "generator root is not a Rust application",
            ))
        }
    }
    fn verify_existing_path(&self, path: &Path) -> Result<()> {
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| CliError::new(ErrorKind::UnsafePath, "path escaped generator root"))?;
        let mut current = self.root.clone();
        for component in relative.components() {
            let Component::Normal(component) = component else {
                return Err(CliError::new(
                    ErrorKind::UnsafePath,
                    "path is not normalized",
                ));
            };
            current.push(component);
            let metadata = fs::symlink_metadata(&current).map_err(|_| {
                CliError::new(ErrorKind::UnsafePath, "project parent must already exist")
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(CliError::new(
                    ErrorKind::UnsafePath,
                    "symbolic links and non-directories are not allowed in project paths",
                ));
            }
        }
        Ok(())
    }
}

fn write_new(path: &Path, contents: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                CliError::new(
                    ErrorKind::AlreadyExists,
                    format!("file already exists: {}", path.display()),
                )
            } else {
                CliError::from_io(error)
            }
        })?;
    let result = file
        .write_all(contents)
        .and_then(|_| file.sync_all())
        .map_err(CliError::from_io);
    drop(file);
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}
fn append_module(path: &Path, module: &str) -> Result<()> {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(CliError::new(
            ErrorKind::UnsafePath,
            "symbolic links are not allowed for generated module files",
        ));
    }
    let declaration = format!("pub mod {module};");
    let existing = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(CliError::from_io(error)),
    };
    if existing.lines().any(|line| line.trim() == declaration) {
        return Err(CliError::new(
            ErrorKind::AlreadyExists,
            "module is already registered",
        ));
    }
    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&declaration);
    updated.push('\n');
    let temporary = path.with_extension("rs.berserk-tmp");
    write_new(&temporary, updated.as_bytes())?;
    if let Err(error) = replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}
fn replace_file(source: &Path, target: &Path) -> Result<()> {
    if !target.exists() {
        return fs::rename(source, target).map_err(CliError::from_io);
    }
    let backup = target.with_extension("rs.framework-backup");
    if backup.exists() {
        return Err(CliError::new(
            ErrorKind::AlreadyExists,
            "generator backup file already exists",
        ));
    }
    fs::rename(target, &backup).map_err(CliError::from_io)?;
    match fs::rename(source, target) {
        Ok(()) => {
            let _ = fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&backup, target);
            Err(CliError::from_io(error))
        }
    }
}
fn validate_relative(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        Err(CliError::new(
            ErrorKind::UnsafePath,
            "path must be a normalized relative path without traversal",
        ))
    } else {
        Ok(())
    }
}
fn validate_package(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 64
        || name.ends_with('-')
        || name.contains("--")
        || !name.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit() && index > 0
                || byte == b'-' && index > 0
        })
    {
        Err(CliError::new(
            ErrorKind::InvalidName,
            "package name must use lowercase ASCII letters, digits, or interior hyphens",
        ))
    } else {
        Ok(())
    }
}
fn validate_type_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 64
        || rust_keyword(name)
        || rust_keyword(&snake_case(name))
        || !name.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphabetic() && (index > 0 || byte.is_ascii_uppercase())
                || byte.is_ascii_digit() && index > 0
        })
    {
        Err(CliError::new(
            ErrorKind::InvalidName,
            "type name must be a PascalCase Rust identifier",
        ))
    } else {
        Ok(())
    }
}
fn validate_snake_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 96
        || !name.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit() && index > 0
                || byte == b'_' && index > 0
        })
        || name.ends_with('_')
        || name.contains("__")
    {
        Err(CliError::new(
            ErrorKind::InvalidName,
            "migration name must be snake_case without repeated or edge underscores",
        ))
    } else {
        Ok(())
    }
}
fn snake_case(name: &str) -> String {
    let mut output = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_uppercase() && index > 0 {
            output.push('_');
        }
        output.push(character.to_ascii_lowercase());
    }
    output
}
fn pascal_case(name: &str) -> String {
    name.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}
fn rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "Self"
            | "self"
            | "super"
            | "crate"
            | "as"
            | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "static"
            | "struct"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
            | "gen"
    )
}

fn migration_source(type_name: &str, timestamp: u64, name: &str) -> String {
    let migration_name = format!("{timestamp}_{name}");
    if let Some(table) = name
        .strip_prefix("create_")
        .and_then(|value| value.strip_suffix("_table"))
    {
        return format!(
            "use berserk::database::{{migrations::{{Column, MigrationPlan, Table}}, Driver, Migration, Result, Statement}};\n\npub struct {type_name};\n\nimpl Migration for {type_name} {{\n    fn name(&self) -> &'static str {{ \"{migration_name}\" }}\n\n    fn up(&self, driver: Driver) -> Result<Vec<Statement>> {{\n        MigrationPlan::new()\n            .create(Table::create(\"{table}\").columns([\n                Column::id(),\n                Column::timestamp(\"created_at\"),\n                Column::timestamp(\"updated_at\"),\n            ]))\n            .compile(driver)\n    }}\n\n    fn down(&self, driver: Driver) -> Result<Vec<Statement>> {{\n        MigrationPlan::new()\n            .table(Table::drop(\"{table}\"))\n            .compile(driver)\n    }}\n}}\n"
        );
    }
    format!(
        "use berserk::database::{{migrations::MigrationPlan, Driver, Migration, Result, Statement}};\n\npub struct {type_name};\n\nimpl Migration for {type_name} {{\n    fn name(&self) -> &'static str {{ \"{migration_name}\" }}\n\n    fn up(&self, driver: Driver) -> Result<Vec<Statement>> {{\n        MigrationPlan::new().compile(driver)\n    }}\n\n    fn down(&self, driver: Driver) -> Result<Vec<Statement>> {{\n        MigrationPlan::new().compile(driver)\n    }}\n}}\n"
    )
}
