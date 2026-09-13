use crate::{CliError, ErrorKind, Result};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    New { path: PathBuf },
    MakeModel { name: String },
    MakeMigration { name: String },
    Migrate(MigrationCommand),
    Help,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationCommand {
    Up,
    Rollback,
    Status,
}

impl Command {
    pub fn parse<I, S>(arguments: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut arguments = arguments.into_iter().map(Into::into);
        let command = arguments.next().unwrap_or_else(|| "help".into());
        let parsed = match command.as_str() {
            "help" | "--help" | "-h" => Self::Help,
            "new" => Self::New {
                path: one(&mut arguments, "new <path>")?.into(),
            },
            "make:model" => Self::MakeModel {
                name: one(&mut arguments, "make:model <Name>")?,
            },
            "make:migration" => Self::MakeMigration {
                name: one(&mut arguments, "make:migration <name>")?,
            },
            "make" => {
                let kind = one(&mut arguments, "make model|migration <name>")?;
                if let Some(name) = kind.strip_prefix("model:") {
                    Self::MakeModel { name: name.into() }
                } else if let Some(name) = kind.strip_prefix("migration:") {
                    Self::MakeMigration { name: name.into() }
                } else {
                    match kind.as_str() {
                        "model" => Self::MakeModel {
                            name: one(&mut arguments, "make model <Name>")?,
                        },
                        "migration" => Self::MakeMigration {
                            name: one(&mut arguments, "make migration <name>")?,
                        },
                        _ => {
                            return Err(CliError::new(
                                ErrorKind::Usage,
                                "make accepts model or migration",
                            ))
                        }
                    }
                }
            }
            "migrate" => Self::Migrate(MigrationCommand::Up),
            "migrate:rollback" => Self::Migrate(MigrationCommand::Rollback),
            "migrate:status" => Self::Migrate(MigrationCommand::Status),
            _ => {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown command: {command}"),
                ))
            }
        };
        if arguments.next().is_some() {
            return Err(CliError::new(
                ErrorKind::Usage,
                "too many command arguments",
            ));
        }
        Ok(parsed)
    }
    pub const fn help() -> &'static str {
        "framework commands:\n  new <path>\n  make:model <Name>\n  make model <Name>\n  make model:<Name>\n  make:migration <name>\n  migrate\n  migrate:rollback\n  migrate:status"
    }
}
fn one(arguments: &mut impl Iterator<Item = String>, usage: &str) -> Result<String> {
    arguments
        .next()
        .ok_or_else(|| CliError::new(ErrorKind::Usage, format!("usage: framework {usage}")))
}

pub trait MigrationExecutor {
    fn execute(&self, command: MigrationCommand) -> Result<String>;
}
pub struct UnsupportedMigrations;
impl MigrationExecutor for UnsupportedMigrations {
    fn execute(&self, _command: MigrationCommand) -> Result<String> {
        Err(CliError::new(ErrorKind::MigrationUnavailable, "migration commands require an application runner configured with a database connection and migration registry"))
    }
}
