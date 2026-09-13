use framework_cli::{
    execute, CliError, Command, ErrorKind, Generator, MigrationCommand, MigrationExecutor,
};
use std::{
    fs,
    path::PathBuf,
    process,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn command_parser_supports_laravel_style_and_spaced_forms() {
    assert_eq!(
        Command::parse(["make:model", "User"]).unwrap(),
        Command::MakeModel {
            name: "User".into()
        }
    );
    assert_eq!(
        Command::parse(["make", "model:User"]).unwrap(),
        Command::MakeModel {
            name: "User".into()
        }
    );
    assert_eq!(
        Command::parse(["make", "migration", "create_users"]).unwrap(),
        Command::MakeMigration {
            name: "create_users".into()
        }
    );
    assert_eq!(
        Command::parse(["migrate:rollback"]).unwrap(),
        Command::Migrate(MigrationCommand::Rollback)
    );
    assert_eq!(
        Command::parse(["unknown"]).unwrap_err().kind(),
        ErrorKind::Usage
    );
}

#[test]
fn project_and_model_generation_refuse_overwrites() {
    let temporary = TemporaryDirectory::new();
    let generator = Generator::at(temporary.path()).unwrap();
    let files = generator
        .new_project(PathBuf::from("demo-api").as_path())
        .unwrap();
    assert_eq!(files.len(), 2);
    assert!(
        fs::read_to_string(temporary.path().join("demo-api/src/main.rs"))
            .unwrap()
            .contains("App::new")
    );
    assert_eq!(
        generator
            .new_project(PathBuf::from("demo-api").as_path())
            .unwrap_err()
            .kind(),
        ErrorKind::AlreadyExists
    );
    let application = Generator::at(temporary.path().join("demo-api")).unwrap();
    application.make_model("UserProfile").unwrap();
    assert!(temporary
        .path()
        .join("demo-api/src/models/user_profile.rs")
        .is_file());
    assert!(
        fs::read_to_string(temporary.path().join("demo-api/src/models/mod.rs"))
            .unwrap()
            .contains("pub mod user_profile;")
    );
    assert_eq!(
        application.make_model("UserProfile").unwrap_err().kind(),
        ErrorKind::AlreadyExists
    );
}

#[test]
fn migration_template_matches_database_contract_and_commands_delegate() {
    let temporary = TemporaryDirectory::new();
    fs::write(
        temporary.path().join("Cargo.toml"),
        "[package]\nname='demo'\nversion='0.1.0'\n",
    )
    .unwrap();
    let generator = Generator::at(temporary.path()).unwrap();
    let files = generator.make_migration("create_users").unwrap();
    let source = fs::read_to_string(&files[0].path).unwrap();
    assert!(source.contains("fn up(&self, _driver: Driver) -> Result<Vec<Statement>>"));
    assert_eq!(
        execute(
            Command::Migrate(MigrationCommand::Status),
            &generator,
            &MigrationFake
        )
        .unwrap(),
        "status"
    );
}

struct MigrationFake;
impl MigrationExecutor for MigrationFake {
    fn execute(&self, command: MigrationCommand) -> framework_cli::Result<String> {
        Ok(match command {
            MigrationCommand::Up => "up",
            MigrationCommand::Rollback => "rollback",
            MigrationCommand::Status => "status",
        }
        .into())
    }
}

struct TemporaryDirectory(PathBuf);
impl TemporaryDirectory {
    fn new() -> Self {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("framework-cli-{}-{time}", process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[allow(dead_code)]
fn assert_cli_error_is_public(error: CliError) -> String {
    error.to_string()
}
