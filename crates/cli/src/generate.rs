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
            fs::create_dir(target.join("src")).map_err(CliError::from_io)?;
            let cargo = format!("[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nberserk = \"0.1\"\n");
            write_new(&target.join("Cargo.toml"), cargo.as_bytes())?;
            write_new(&target.join("src/main.rs"), b"use berserk::{App, Response, Result};\n\nfn main() -> Result<()> {\n    let mut app = App::new();\n    app.get(\"/\", |_| Response::text(\"Hello, world!\"))?;\n    app.listen(\"127.0.0.1:3000\")\n}\n")?;
            Ok(vec![
                GeneratedFile {
                    path: target.join("Cargo.toml"),
                },
                GeneratedFile {
                    path: target.join("src/main.rs"),
                },
            ])
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&target);
        }
        result
    }
    pub fn make_model(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_type_name(name)?;
        let module = snake_case(name);
        let directory = self.safe_directory("src/models")?;
        let model = directory.join(format!("{module}.rs"));
        let index = directory.join("mod.rs");
        if model.exists() {
            return Err(CliError::new(
                ErrorKind::AlreadyExists,
                "model file already exists",
            ));
        }
        let source = format!("#[derive(Clone, Debug, Eq, PartialEq)]\npub struct {name} {{\n    pub id: i64,\n}}\n\n// Implement berserk::database::Model after defining the table's complete fields.\n");
        write_new(&model, source.as_bytes())?;
        if let Err(error) = append_module(&index, &module) {
            let _ = fs::remove_file(&model);
            return Err(error);
        }
        Ok(vec![
            GeneratedFile { path: model },
            GeneratedFile { path: index },
        ])
    }
    pub fn make_migration(&self, name: &str) -> Result<Vec<GeneratedFile>> {
        self.ensure_application()?;
        validate_snake_name(name)?;
        let directory = self.safe_directory("migrations")?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CliError::new(ErrorKind::Clock, "system clock is before Unix epoch"))?
            .as_secs();
        let path = directory.join(format!("{timestamp}_{name}.rs"));
        let type_name = pascal_case(name);
        let source = format!("use berserk::database::{{Driver, Migration, Result, Statement}};\n\npub struct {type_name};\n\nimpl Migration for {type_name} {{\n    fn name(&self) -> &'static str {{ \"{timestamp}_{name}\" }}\n    fn up(&self, _driver: Driver) -> Result<Vec<Statement>> {{\n        Ok(vec![Statement::new(\"-- write forward migration SQL\")])\n    }}\n    fn down(&self, _driver: Driver) -> Result<Vec<Statement>> {{\n        Ok(vec![Statement::new(\"-- write rollback migration SQL\")])\n    }}\n}}\n");
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
            "model module is already registered",
        ));
    }
    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&declaration);
    updated.push('\n');
    let temporary = path.with_extension("rs.framework-tmp");
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
        || !name.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphabetic() && (index > 0 || byte.is_ascii_uppercase())
                || byte.is_ascii_digit() && index > 0
        })
    {
        Err(CliError::new(
            ErrorKind::InvalidName,
            "model name must be a PascalCase Rust identifier",
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
    name == "Self"
}
