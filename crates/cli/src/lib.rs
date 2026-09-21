//! Safe, Laravel-inspired project generators and migration command hooks.
#![forbid(unsafe_code)]

mod command;
mod error;
mod generate;

pub use command::{Command, MigrationCommand, MigrationExecutor, UnsupportedMigrations};
pub use error::{CliError, ErrorKind, Result};
pub use generate::{GeneratedFile, Generator};

pub fn execute(
    command: Command,
    generator: &Generator,
    migrations: &dyn MigrationExecutor,
) -> Result<String> {
    match command {
        Command::New { path } => generator
            .new_project(&path)
            .map(|files| summary("project", &files)),
        Command::Serve => serve(generator),
        Command::MakeController {
            name,
            resource,
            model,
            request,
        } => {
            let files = if resource {
                let model = model.ok_or_else(|| {
                    CliError::new(ErrorKind::Usage, "resource controller requires --model")
                })?;
                let request = request.ok_or_else(|| {
                    CliError::new(ErrorKind::Usage, "resource controller requires --request")
                })?;
                generator.make_resource_controller(&name, &model, &request)?
            } else {
                generator.make_controller(&name)?
            };
            Ok(summary("controller", &files))
        }
        Command::MakeRequest { name } => generator
            .make_request(&name)
            .map(|files| summary("request", &files)),
        Command::MakeMiddleware { name } => generator
            .make_middleware(&name)
            .map(|files| summary("middleware", &files)),
        Command::MakeResource { name } => generator
            .make_resource(&name)
            .map(|files| summary("resource", &files)),
        Command::MakePolicy { name } => generator
            .make_policy(&name)
            .map(|files| summary("policy", &files)),
        Command::MakeModel { name } => generator
            .make_model(&name)
            .map(|files| summary("model", &files)),
        Command::MakeMigration { name } => generator
            .make_migration(&name)
            .map(|files| summary("migration", &files)),
        Command::Migrate(operation) => migrations.execute(operation),
        Command::Help => Ok(Command::help().into()),
    }
}

fn serve(generator: &Generator) -> Result<String> {
    if !generator.root().join("Cargo.toml").is_file() {
        return Err(CliError::new(
            ErrorKind::UnsafePath,
            "run berserk serve from an application containing Cargo.toml",
        ));
    }
    let status = std::process::Command::new("cargo")
        .arg("run")
        .current_dir(generator.root())
        .status()
        .map_err(CliError::from_io)?;
    if !status.success() {
        return Err(CliError::new(
            ErrorKind::Process,
            format!("cargo run failed: {status}"),
        ));
    }
    Ok("application stopped".into())
}

fn summary(kind: &str, files: &[GeneratedFile]) -> String {
    format!(
        "generated {kind}: {}",
        files
            .iter()
            .map(|file| file.path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}
