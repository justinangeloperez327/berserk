use crate::{CliError, ErrorKind, Result};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    New { path: PathBuf },
    Serve,
    MakeModel { name: String },
    MakeController {
        name: String,
        resource: bool,
        model: Option<String>,
        request: Option<String>,
    },
    MakeRequest { name: String },
    MakeMiddleware { name: String },
    MakeResource { name: String },
    MakePolicy { name: String },
    MakeMigration { name: String },
    Migrate(MigrationCommand),
    Help,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationCommand {
    Up,
    Rollback,
    Status,
    DryRun,
    Reset,
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
            "serve" => Self::Serve,
            "make:controller" => controller(
                &mut arguments,
                "make:controller <Name> [--resource --model <Model> --request <Request>]",
            )?,
            "make:request" => Self::MakeRequest {
                name: one(&mut arguments, "make:request <Name>")?,
            },
            "make:middleware" => Self::MakeMiddleware {
                name: one(&mut arguments, "make:middleware <Name>")?,
            },
            "make:resource" => Self::MakeResource {
                name: one(&mut arguments, "make:resource <Name>")?,
            },
            "make:policy" => Self::MakePolicy {
                name: one(&mut arguments, "make:policy <Name>")?,
            },
            "make:model" => Self::MakeModel {
                name: one(&mut arguments, "make:model <Name>")?,
            },
            "make:migration" => Self::MakeMigration {
                name: one(&mut arguments, "make:migration <name>")?,
            },
            "make" => {
                let kind = one(&mut arguments, "make <kind> <name>")?;
                if let Some(name) = kind.strip_prefix("model:") {
                    Self::MakeModel { name: name.into() }
                } else if let Some(name) = kind.strip_prefix("migration:") {
                    Self::MakeMigration { name: name.into() }
                } else {
                    match kind.as_str() {
                        "controller" => controller(
                            &mut arguments,
                            "make controller <Name> [--resource --model <Model> --request <Request>]",
                        )?,
                        "request" => Self::MakeRequest {
                            name: one(&mut arguments, "make request <Name>")?,
                        },
                        "middleware" => Self::MakeMiddleware {
                            name: one(&mut arguments, "make middleware <Name>")?,
                        },
                        "resource" => Self::MakeResource {
                            name: one(&mut arguments, "make resource <Name>")?,
                        },
                        "policy" => Self::MakePolicy {
                            name: one(&mut arguments, "make policy <Name>")?,
                        },
                        "model" => Self::MakeModel {
                            name: one(&mut arguments, "make model <Name>")?,
                        },
                        "migration" => Self::MakeMigration {
                            name: one(&mut arguments, "make migration <name>")?,
                        },
                        _ => {
                            return Err(CliError::new(
                                ErrorKind::Usage,
                                "make accepts model, controller, request, middleware, resource, policy, or migration",
                            ))
                        }
                    }
                }
            }
            "migrate" => match arguments.next().as_deref() {
                None => Self::Migrate(MigrationCommand::Up),
                Some("--dry-run") => Self::Migrate(MigrationCommand::DryRun),
                Some(_) => {
                    return Err(CliError::new(
                        ErrorKind::Usage,
                        "usage: berserk migrate [--dry-run]",
                    ))
                }
            },
            "migrate:rollback" => Self::Migrate(MigrationCommand::Rollback),
            "migrate:status" => Self::Migrate(MigrationCommand::Status),
            "migrate:reset" => Self::Migrate(MigrationCommand::Reset),
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
        "berserk commands:\n  new <path>\n  serve\n  make:controller <Name>\n  make:controller <Name> --resource --model <Model> --request <Request>\n  make:request <Name>\n  make:middleware <Name>\n  make:resource <Name>\n  make:policy <Name>\n  make:model <Name>\n  make model <Name>\n  make model:<Name>\n  make:migration <name>\n  migrate\n  migrate --dry-run\n  migrate:rollback\n  migrate:reset\n  migrate:status"
    }
}

fn controller(
    arguments: &mut impl Iterator<Item = String>,
    usage: &str,
) -> Result<Command> {
    let name = one(arguments, usage)?;
    let mut resource = false;
    let mut model = None;
    let mut request = None;

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--resource" if !resource => resource = true,
            "--resource" => {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "controller --resource may only be specified once",
                ))
            }
            "--model" => {
                if model.is_some() {
                    return Err(CliError::new(
                        ErrorKind::Usage,
                        "controller --model may only be specified once",
                    ));
                }
                model = Some(one(arguments, usage)?);
            }
            "--request" => {
                if request.is_some() {
                    return Err(CliError::new(
                        ErrorKind::Usage,
                        "controller --request may only be specified once",
                    ));
                }
                request = Some(one(arguments, usage)?);
            }
            _ => {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown controller option: {argument}"),
                ))
            }
        }
    }

    if resource && (model.is_none() || request.is_none()) {
        return Err(CliError::new(
            ErrorKind::Usage,
            format!("usage: berserk {usage}"),
        ));
    }
    if !resource && (model.is_some() || request.is_some()) {
        return Err(CliError::new(
            ErrorKind::Usage,
            "--model and --request require --resource",
        ));
    }

    Ok(Command::MakeController {
        name,
        resource,
        model,
        request,
    })
}

fn one(arguments: &mut impl Iterator<Item = String>, usage: &str) -> Result<String> {
    arguments
        .next()
        .ok_or_else(|| CliError::new(ErrorKind::Usage, format!("usage: berserk {usage}")))
}

pub trait MigrationExecutor {
    fn execute(&self, command: MigrationCommand) -> Result<String>;
}

pub struct UnsupportedMigrations;

impl MigrationExecutor for UnsupportedMigrations {
    fn execute(&self, _command: MigrationCommand) -> Result<String> {
        Err(CliError::new(
            ErrorKind::MigrationUnavailable,
            "migration commands require an application runner configured with a database connection and migration registry",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_controller() {
        assert_eq!(
            Command::parse(["make:controller", "UserController"]).unwrap(),
            Command::MakeController {
                name: "UserController".into(),
                resource: false,
                model: None,
                request: None,
            }
        );
    }

    #[test]
    fn parses_resource_controller() {
        assert_eq!(
            Command::parse([
                "make:controller",
                "UserController",
                "--resource",
                "--model",
                "User",
                "--request",
                "UserInput",
            ])
            .unwrap(),
            Command::MakeController {
                name: "UserController".into(),
                resource: true,
                model: Some("User".into()),
                request: Some("UserInput".into()),
            }
        );
    }

    #[test]
    fn resource_controller_requires_model_and_request() {
        assert!(Command::parse([
            "make:controller",
            "UserController",
            "--resource",
            "--model",
            "User",
        ])
        .is_err());
    }
}
