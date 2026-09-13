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
