use crate::{
    naming::{validate_package_name, validate_rust_version, validate_version},
    routes_source, RoutesSpec,
};

/// One generated file in a conventional Berserk application skeleton.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationFile {
    path: String,
    content: String,
}

impl ApplicationFile {
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

/// Source-level specification for a conventional Berserk application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationSpec {
    package: String,
    berserk_version: String,
    rust_version: String,
    routes: RoutesSpec,
}

impl ApplicationSpec {
    pub fn new(
        package: impl Into<String>,
        berserk_version: impl Into<String>,
        rust_version: impl Into<String>,
    ) -> Self {
        Self {
            package: package.into(),
            berserk_version: berserk_version.into(),
            rust_version: rust_version.into(),
            routes: RoutesSpec::application(),
        }
    }

    pub fn routes(mut self, routes: RoutesSpec) -> Self {
        self.routes = routes;
        self
    }

    pub fn package(&self) -> &str {
        &self.package
    }

    pub fn berserk_version(&self) -> &str {
        &self.berserk_version
    }

    pub fn rust_version(&self) -> &str {
        &self.rust_version
    }

    pub fn route_spec(&self) -> &RoutesSpec {
        &self.routes
    }
}

/// Generate the complete conventional source skeleton used by `berserk new`.
///
/// The returned paths are normalized relative application paths. This function
/// performs no filesystem I/O; callers decide where and how the files are
/// written.
pub fn application_files(spec: &ApplicationSpec) -> syn::Result<Vec<ApplicationFile>> {
    validate_package_name(spec.package())?;
    validate_version(spec.berserk_version(), "Berserk version")?;
    validate_rust_version(spec.rust_version())?;

    let cargo = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"{}\"\n\n[dependencies]\nberserk = {{ version = \"{}\", features = [\"claw\"] }}\n",
        spec.package(),
        spec.rust_version(),
        spec.berserk_version(),
    );
    let routes = routes_source(spec.route_spec())?;

    Ok(vec![
        ApplicationFile::new("Cargo.toml", cargo),
        ApplicationFile::new("src/main.rs", main_source()),
        ApplicationFile::new("src/config/mod.rs", config_source()),
        ApplicationFile::new("src/app/routes.rs", routes),
        ApplicationFile::new(
            "src/app/mod.rs",
            "pub mod controllers;\npub mod middleware;\npub mod models;\npub mod policies;\npub mod resources;\npub mod routes;\npub mod validations;\n",
        ),
        ApplicationFile::new("src/app/controllers/mod.rs", ""),
        ApplicationFile::new("src/app/models/mod.rs", ""),
        ApplicationFile::new("src/app/validations/mod.rs", ""),
        ApplicationFile::new("src/app/middleware/mod.rs", ""),
        ApplicationFile::new("src/app/resources/mod.rs", ""),
        ApplicationFile::new("src/app/policies/mod.rs", ""),
        ApplicationFile::new("src/database/mod.rs", "pub mod migrations;\n"),
        ApplicationFile::new("src/database/migrations/mod.rs", ""),
    ])
}

fn main_source() -> &'static str {
    r#"mod app;
mod config;
mod database;

use berserk::{App, HandleErrors, RequestId, Result};

fn main() -> Result<()> {
    let config = config::AppConfig::from_env()?;
    let address = config.address;
    let mut app = App::new();
    app.configure(config)?;
    app.middleware(RequestId);
    app.middleware(HandleErrors);
    app::routes::register(&mut app)?;
    app.listen(address)
}
"#
}

fn config_source() -> &'static str {
    r#"use berserk::{ConfigError, Validate};
use std::{env, net::SocketAddr};

pub struct AppConfig {
    pub address: SocketAddr,
    pub name: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let address = value("BERSERK_ADDRESS", "127.0.0.1:3000")?
            .parse()
            .map_err(|_| ConfigError::new("BERSERK_ADDRESS", "expected an IP address and port"))?;
        let config = Self {
            address,
            name: value("APP_NAME", "Berserk API")?,
        };
        config.validate()?;
        Ok(config)
    }
}

impl Validate for AppConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.name.trim().is_empty() {
            return Err(ConfigError::new("APP_NAME", "must not be empty"));
        }
        if self.address.port() == 0 {
            return Err(ConfigError::new("BERSERK_ADDRESS", "port must be nonzero"));
        }
        Ok(())
    }
}

fn value(name: &'static str, default: &str) -> Result<String, ConfigError> {
    match env::var(name) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(default.into()),
        Err(env::VarError::NotUnicode(_)) => Err(ConfigError::new(name, "must be UTF-8")),
    }
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RouteSpec;

    #[test]
    fn conventional_application_contains_the_complete_skeleton() {
        let files = application_files(&ApplicationSpec::new("my-api", "1.0.0", "1.88")).unwrap();

        assert_eq!(files.len(), 13);
        assert!(files.iter().any(|file| file.path() == "Cargo.toml"));
        assert!(files.iter().any(|file| file.path() == "src/main.rs"));
        assert!(files.iter().any(|file| file.path() == "src/config/mod.rs"));
        assert!(files.iter().any(|file| file.path() == "src/app/routes.rs"));
        assert!(files
            .iter()
            .any(|file| file.path() == "src/database/migrations/mod.rs"));

        for file in &files {
            if file.path().ends_with(".rs") && !file.content().is_empty() {
                syn::parse_file(file.content()).unwrap();
            }
        }
    }

    #[test]
    fn application_routes_can_be_replaced_by_tools() {
        let routes = RoutesSpec::new().route(RouteSpec::crud("/users", "UserController"));
        let files =
            application_files(&ApplicationSpec::new("my-api", "1.0.0", "1.88").routes(routes))
                .unwrap();

        let routes = files
            .iter()
            .find(|file| file.path() == "src/app/routes.rs")
            .unwrap();
        assert!(routes
            .content()
            .contains("app.route().crud(\"/users\", UserController)?;"));
        assert!(!routes.content().contains("\"/health\""));
    }

    #[test]
    fn application_metadata_rejects_source_injection() {
        assert!(application_files(&ApplicationSpec::new("bad\nname", "1.0.0", "1.88")).is_err());
        assert!(application_files(&ApplicationSpec::new(
            "good-name",
            "1.0.0\"\nmalicious = true",
            "1.88"
        ))
        .is_err());
        assert!(application_files(&ApplicationSpec::new("good-name", "1.0.0", "nightly")).is_err());
    }
}
