use berserk_cli::{
    execute, CliError, Command, ErrorKind, Generator, MigrationCommand, MigrationExecutor,
};
use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    process,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_TEMP_DIRECTORY_ID: AtomicU64 = AtomicU64::new(0);

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
        Command::parse(["migrate", "--dry-run"]).unwrap(),
        Command::Migrate(MigrationCommand::DryRun)
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
fn generated_project_targets_the_cli_release_and_minimum_rust_version() {
    let temporary = TemporaryDirectory::new();
    let generator = Generator::at(temporary.path()).unwrap();
    generator
        .new_project(std::path::Path::new("versioned-api"))
        .unwrap();
    let manifest = fs::read_to_string(temporary.path().join("versioned-api/Cargo.toml")).unwrap();

    assert!(manifest.lines().any(|line| {
        line == format!(
            "berserk = {{ version = \"{}\", features = [\"claw\"] }}",
            env!("CARGO_PKG_VERSION")
        )
    }));
    assert!(manifest
        .lines()
        .any(|line| { line == format!("rust-version = \"{}\"", env!("CARGO_PKG_RUST_VERSION")) }));
    // The consumer's own version is independent of its framework dependency.
    assert!(manifest.lines().any(|line| line == "version = \"0.1.0\""));
}

#[test]
fn project_and_model_generation_refuse_overwrites() {
    let temporary = TemporaryDirectory::new();
    let generator = Generator::at(temporary.path()).unwrap();
    let files = generator
        .new_project(PathBuf::from("demo-api").as_path())
        .unwrap();
    assert_eq!(files.len(), 8);
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
    let files = generator.make_migration("create_users_table").unwrap();
    let source = fs::read_to_string(&files[0].path).unwrap();
    assert!(source.contains("MigrationPlan::new()"));
    assert!(source.contains("Table::create(\"users\")"));
    assert!(source.contains("Column::timestamp(\"created_at\")"));
    assert!(source.contains("Table::drop(\"users\")"));
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

#[test]
fn temporary_directories_are_unique_under_concurrent_creation() {
    let directories = (0..32)
        .map(|_| thread::spawn(TemporaryDirectory::new))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();

    let unique_paths = directories
        .iter()
        .map(|directory| directory.path().to_path_buf())
        .collect::<HashSet<_>>();

    assert_eq!(unique_paths.len(), directories.len());
}

struct MigrationFake;
impl MigrationExecutor for MigrationFake {
    fn execute(&self, command: MigrationCommand) -> berserk_cli::Result<String> {
        Ok(match command {
            MigrationCommand::Up => "up",
            MigrationCommand::Rollback => "rollback",
            MigrationCommand::Status => "status",
            MigrationCommand::DryRun => "dry-run",
        }
        .into())
    }
}

struct TemporaryDirectory(PathBuf);
impl TemporaryDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_TEMP_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("berserk-cli-{}-{nonce}-{sequence}", process::id()));
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

#[test]
fn generator_rejects_type_names_that_become_reserved_modules() {
    let temporary = TemporaryDirectory::new();
    let root = temporary.path();
    let generator = Generator::at(root).unwrap();
    generator.new_project(std::path::Path::new("app")).unwrap();
    let generator = Generator::at(root.join("app")).unwrap();
    for name in ["Type", "Crate", "Async", "Self", "Super"] {
        assert!(generator.make_model(name).is_err(), "accepted {name}");
    }
}
