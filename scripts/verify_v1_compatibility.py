#!/usr/bin/env python3
"""Compile Berserk v1.0 public API compatibility fixtures as external consumers."""
from pathlib import Path
import os
import re
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
FRAMEWORK = (ROOT / "crates/framework").as_posix()
VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]

CASES = {
    "core": {
        "features": [],
        "source": r'''use berserk::{response, App, FormRequest, FromJson, Json, Request, Response, Result, ValidationErrors, ValidationResult};

struct Input { name: String }
impl FromJson for Input {
    fn from_json(value: &Json) -> std::result::Result<Self, ValidationErrors> {
        Ok(Self { name: value.get("name").and_then(Json::as_str).unwrap_or_default().to_owned() })
    }
}
impl FormRequest for Input {
    fn sanitize(&mut self) { self.name = self.name.trim().to_owned(); }
    fn validate(&self) -> ValidationResult { Ok(()) }
}
fn show(id: u64) -> Result<Response> { response().text(format!("User {id}")) }
fn accepted(input: Input) -> Result<Response> { response().text(input.name) }
fn main() -> Result<()> {
    let mut app = App::new();
    let mut route = app.route();
    route.get("/users/{id}", show)?;
    route.post("/users", accepted)?;
    drop(route);
    let _ = app.path_for("missing", &[]);
    let _handler: fn(Request) -> Result<Response> = |request| response().bytes(request.body().to_vec());
    Ok(())
}
''',
    },
    "claw": {
        "features": ["claw", "sqlite"],
        "source": r'''use berserk::{response, CrudController, FormRequest, FromJson, Json, Model, Response, Result, ValidationErrors, ValidationResult};
use berserk::claw::{Collection, Direction, IntoInsert, IntoUpdate, Value};

#[derive(Model)]
#[table("users")]
struct User {
    #[primary_key]
    id: i64,
    #[fillable]
    name: String,
}
struct UserInput { name: String }
impl FromJson for UserInput {
    fn from_json(value: &Json) -> std::result::Result<Self, ValidationErrors> {
        Ok(Self { name: value.get("name").and_then(Json::as_str).unwrap_or_default().to_owned() })
    }
}
impl FormRequest for UserInput { fn validate(&self) -> ValidationResult { Ok(()) } }
impl IntoInsert<User> for UserInput {
    fn into_insert(self) -> berserk::claw::Result<Vec<(&'static str, Value)>> { Ok(vec![("name", self.name.into())]) }
}
impl IntoUpdate<User> for UserInput {
    fn into_update(self) -> berserk::claw::Result<Vec<(&'static str, Value)>> { Ok(vec![("name", self.name.into())]) }
}
struct Users;
impl CrudController for Users {
    type Model = User;
    type Create = UserInput;
    type Update = UserInput;
    fn index(&self) -> Result<Response> { response().collection(User::all()?) }
    fn store(&self, input: Self::Create) -> Result<Response> { response().status(201).json(User::create(input)?) }
    fn show(&self, model: Self::Model) -> Result<Response> { response().json(model) }
    fn update(&self, mut model: Self::Model, input: Self::Update) -> Result<Response> { model.update(input)?; response().json(model) }
    fn destroy(&self, model: Self::Model) -> Result<Response> { model.delete()?; response().no_content() }
}
fn compile_surface() {
    let _: Collection<User> = Collection::new();
    let _ = User::query().where_("name", "Ada").where_not_null("name").order_by("id", Direction::Asc).limit(10).offset(0);
    let _ = User::find(1_i64);
    let _ = User::find_or_fail(1_i64);
}
fn main() { compile_surface(); }
''',
    },
    "integrations": {
        "features": ["auth", "view", "claw", "sqlite"],
        "source": r'''use berserk::{App, Request, Result};
use berserk::prelude::*;
fn accepts_request(_request: Request) {}
fn main() -> Result<()> {
    let app = App::new();
    let _metrics = Metrics::new();
    let _health = HealthRegistry::new();
    let _ = &app;
    let _ = accepts_request as fn(Request);
    Ok(())
}
''',
    },
}

def run(*args, cwd=ROOT, env=None):
    subprocess.run(args, cwd=cwd, env=env, check=True)

def manifest(features):
    feature_text = ""
    if features:
        feature_text = ", features = [" + ", ".join(f'"{f}"' for f in features) + "]"
    return f'''[package]
name = "berserk-v1-compat"
version = "0.0.0"
edition = "2021"
rust-version = "1.88"
publish = false

[workspace]

[dependencies]
berserk = {{ version = "={VERSION}", path = "{FRAMEWORK}", default-features = false{feature_text} }}
'''

def main():
    with tempfile.TemporaryDirectory(prefix="berserk-v1-compat-") as temporary:
        base = Path(temporary)
        for name, case in CASES.items():
            consumer = base / name
            (consumer / "src").mkdir(parents=True)
            (consumer / "Cargo.toml").write_text(manifest(case["features"]))
            (consumer / "src/main.rs").write_text(case["source"])
            env = os.environ.copy()
            env["CARGO_TARGET_DIR"] = str(base / "target")
            run("cargo", "check", "--manifest-path", str(consumer / "Cargo.toml"), env=env)

if __name__ == "__main__":
    main()
