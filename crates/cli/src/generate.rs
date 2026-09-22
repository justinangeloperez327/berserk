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
            let spec = berserk_codegen::ApplicationSpec::new(
                package,
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_RUST_VERSION"),
            );
            let sources = berserk_codegen::application_files(&spec).map_err(|error| {
                CliError::new(
                    ErrorKind::Process,
                    format!("could not generate application skeleton: {error}"),
                )
            })?;
            let mut files = Vec::with_capacity(sources.len());

            for source in sources {
                let path = target.join(source.path());
                let parent = path.parent().ok_or_else(|| {
                    CliError::new(ErrorKind::UnsafePath, "generated file has no parent")
                })?;
                fs::create_dir_all(parent).map_err(CliError::from_io)?;
                write_new(&path, source.content().as_bytes())?;
                files.push(GeneratedFile { path });
            }

            Ok(files)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&target);
        }
        result
    }
    pub fn make_crud(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_type_name(name)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CliError::new(ErrorKind::Clock, "system clock is before Unix epoch"))?
            .as_secs();
        let spec = berserk_codegen::ModuleSpec::crud(name, timestamp)
            .map_err(|error| CliError::new(ErrorKind::InvalidName, error.to_string()))?;
        let sources = berserk_codegen::module_files(&spec)
            .map_err(|error| CliError::new(ErrorKind::InvalidName, error.to_string()))?;

        let routes_path = self.root.join("src/app/routes.rs");
        self.validate_generated_file(&routes_path)?;
        let original_routes = fs::read_to_string(&routes_path).map_err(CliError::from_io)?;
        let updated_routes = berserk_codegen::register_route_source(&original_routes, spec.route())
            .map_err(|error| CliError::new(ErrorKind::Process, error.to_string()))?;

        let registrations = [
            (
                self.root.join("src/app/models/mod.rs"),
                format!("pub mod {};", snake_case(spec.model().name())),
            ),
            (
                self.root.join("src/app/validations/mod.rs"),
                format!("pub mod {};", snake_case(spec.request().name())),
            ),
            (
                self.root.join("src/app/controllers/mod.rs"),
                format!("pub mod {};", snake_case(spec.controller().name())),
            ),
        ];

        let mut index_updates = Vec::new();
        for (path, declaration) in registrations {
            self.validate_generated_file(&path)?;
            let original = fs::read_to_string(&path).map_err(CliError::from_io)?;
            if original.lines().any(|line| line.trim() == declaration) {
                return Err(CliError::new(
                    ErrorKind::AlreadyExists,
                    format!("module is already registered: {declaration}"),
                ));
            }
            let mut updated = original.clone();
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(&declaration);
            updated.push('\n');
            index_updates.push((path, original, updated));
        }

        let migration_index = self.root.join("src/database/migrations/mod.rs");
        self.validate_generated_file(&migration_index)?;
        let migration_original = fs::read_to_string(&migration_index).map_err(CliError::from_io)?;
        let migration_file = format!(
            "{}_{}.rs",
            spec.migration().timestamp(),
            spec.migration().name()
        );
        let migration_module = format!(
            "{}_{}",
            spec.migration().name(),
            spec.migration().timestamp()
        );
        let migration_declaration =
            format!("#[path = \"{migration_file}\"]\npub mod {migration_module};");
        if migration_original.contains(&migration_declaration) {
            return Err(CliError::new(
                ErrorKind::AlreadyExists,
                "migration is already registered",
            ));
        }
        let mut migration_updated = migration_original.clone();
        if !migration_updated.is_empty() && !migration_updated.ends_with('\n') {
            migration_updated.push('\n');
        }
        migration_updated.push_str(&migration_declaration);
        migration_updated.push('\n');
        index_updates.push((migration_index, migration_original, migration_updated));

        let mut source_paths = Vec::with_capacity(sources.len());
        for source in &sources {
            let relative = Path::new(source.path());
            validate_relative(relative)?;
            let parent = relative.parent().ok_or_else(|| {
                CliError::new(ErrorKind::UnsafePath, "generated module file has no parent")
            })?;
            let directory = self.safe_directory(parent.to_str().ok_or_else(|| {
                CliError::new(ErrorKind::UnsafePath, "generated module path must be UTF-8")
            })?)?;
            let file_name = relative.file_name().ok_or_else(|| {
                CliError::new(ErrorKind::UnsafePath, "generated module file has no name")
            })?;
            let path = directory.join(file_name);
            if path.exists() {
                return Err(CliError::new(
                    ErrorKind::AlreadyExists,
                    format!("file already exists: {}", path.display()),
                ));
            }
            source_paths.push(path);
        }

        let mut created = Vec::new();
        for (source, path) in sources.iter().zip(&source_paths) {
            if let Err(error) = write_new(path, source.content().as_bytes()) {
                for path in &created {
                    let _ = fs::remove_file(path);
                }
                return Err(error);
            }
            created.push(path.clone());
        }

        let mut replaced = Vec::new();
        for (path, original, updated) in &index_updates {
            if let Err(error) = replace_contents(path, updated) {
                rollback_crud(&created, &replaced);
                return Err(error);
            }
            replaced.push((path.clone(), original.clone()));
        }
        if let Err(error) = replace_contents(&routes_path, &updated_routes) {
            rollback_crud(&created, &replaced);
            return Err(error);
        }

        let mut files = created
            .into_iter()
            .map(|path| GeneratedFile { path })
            .collect::<Vec<_>>();
        files.extend(
            index_updates
                .into_iter()
                .map(|(path, _, _)| GeneratedFile { path }),
        );
        files.push(GeneratedFile { path: routes_path });
        Ok(files)
    }

    pub fn make_model(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_type_name(name)?;
        let source = berserk_codegen::model_source(&berserk_codegen::ModelSpec::new(name))
            .map_err(|error| CliError::new(ErrorKind::InvalidName, error.to_string()))?;
        self.make_source_type(name, "app/models", &source)
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
        self.ensure_application()?;
        validate_type_name(name)?;
        let source =
            berserk_codegen::middleware_source(&berserk_codegen::MiddlewareSpec::new(name))
                .map_err(|error| CliError::new(ErrorKind::InvalidName, error.to_string()))?;
        self.make_source_type(name, "app/middleware", &source)
    }

    pub fn make_resource(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_type_name(name)?;
        let source = berserk_codegen::resource_source(&berserk_codegen::ResourceSpec::new(name))
            .map_err(|error| CliError::new(ErrorKind::InvalidName, error.to_string()))?;
        self.make_source_type(name, "app/resources", &source)
    }

    pub fn make_policy(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_type_name(name)?;
        let source = berserk_codegen::policy_source(&berserk_codegen::PolicySpec::new(name))
            .map_err(|error| CliError::new(ErrorKind::InvalidName, error.to_string()))?;
        self.make_source_type(name, "app/policies", &source)
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
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CliError::new(ErrorKind::Clock, "system clock is before Unix epoch"))?
            .as_secs();
        let source = berserk_codegen::migration_source(&berserk_codegen::MigrationSpec::new(
            name, timestamp,
        ))
        .map_err(|error| CliError::new(ErrorKind::InvalidName, error.to_string()))?;
        let directory = self.safe_directory("src/database/migrations")?;
        let file_name = format!("{timestamp}_{name}.rs");
        let path = directory.join(&file_name);
        write_new(&path, source.as_bytes())?;
        let index = directory.join("mod.rs");
        let module = format!("{name}_{timestamp}");
        if let Err(error) = append_migration_module(&index, &file_name, &module) {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        Ok(vec![GeneratedFile { path }, GeneratedFile { path: index }])
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
    fn validate_generated_file(&self, path: &Path) -> Result<()> {
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| CliError::new(ErrorKind::UnsafePath, "generated file escaped its root"))?;
        let parent = relative.parent().ok_or_else(|| {
            CliError::new(
                ErrorKind::UnsafePath,
                "generated file has no parent directory",
            )
        })?;
        let parent = parent.to_str().ok_or_else(|| {
            CliError::new(ErrorKind::UnsafePath, "generated file path must be UTF-8")
        })?;
        let directory = self.safe_directory(parent)?;
        let expected =
            directory.join(relative.file_name().ok_or_else(|| {
                CliError::new(ErrorKind::UnsafePath, "generated file has no name")
            })?);
        let metadata = fs::symlink_metadata(&expected).map_err(CliError::from_io)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(CliError::new(
                ErrorKind::UnsafePath,
                "generated application file must be a regular file",
            ));
        }
        Ok(())
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

fn append_migration_module(path: &Path, file: &str, module: &str) -> Result<()> {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(CliError::new(
            ErrorKind::UnsafePath,
            "symbolic links are not allowed for generated migration modules",
        ));
    }
    let declaration = format!("#[path = \"{file}\"]\npub mod {module};");
    let existing = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(CliError::from_io(error)),
    };
    if existing.contains(&declaration) {
        return Err(CliError::new(
            ErrorKind::AlreadyExists,
            "migration module is already registered",
        ));
    }
    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&declaration);
    updated.push('\n');
    replace_contents(path, &updated)
}

fn replace_contents(path: &Path, contents: &str) -> Result<()> {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(CliError::new(
            ErrorKind::UnsafePath,
            "symbolic links are not allowed for generated files",
        ));
    }
    let temporary = path.with_extension("rs.berserk-tmp");
    if temporary.exists() {
        return Err(CliError::new(
            ErrorKind::AlreadyExists,
            "generator temporary file already exists",
        ));
    }
    write_new(&temporary, contents.as_bytes())?;
    if let Err(error) = replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

fn rollback_crud(created: &[PathBuf], replaced: &[(PathBuf, String)]) {
    for path in created {
        let _ = fs::remove_file(path);
    }
    for (path, original) in replaced.iter().rev() {
        let _ = replace_contents(path, original);
    }
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
