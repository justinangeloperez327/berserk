#!/usr/bin/env python3
"""Compile every application generator in an independent consumer project."""
from pathlib import Path
import os
import re
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]

def run(*args, cwd=ROOT):
    subprocess.run(args, cwd=cwd, check=True)


def main():
    workspace_package = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]
    run("cargo", "build", "-p", "berserk-cli", "--bin", "berserk")
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    if not target.is_absolute():
        target = ROOT / target
    cli = target / "debug" / ("berserk.exe" if os.name == "nt" else "berserk")
    with tempfile.TemporaryDirectory(prefix="berserk-generators-") as temporary:
        base = Path(temporary)
        run(str(cli), "new", "consumer", cwd=base)
        consumer = base / "consumer"
        manifest = consumer / "Cargo.toml"
        generated_text = manifest.read_text()
        generated = tomllib.loads(generated_text)
        framework = generated["dependencies"]["berserk"]
        if framework.get("version") != workspace_package["version"]:
            raise RuntimeError("generated Berserk dependency does not match the workspace version")
        if "path" in framework or "git" in framework:
            raise RuntimeError("generated Berserk dependency must target the package registry")
        if generated["package"]["rust-version"] != workspace_package["rust-version"]:
            raise RuntimeError("generated minimum Rust version does not match the workspace")
        dependency = (ROOT / "crates/framework").as_posix()
        manifest_text, replacements = re.subn(
            r"^berserk\s*=\s*.+$",
            lambda _: f'berserk = {{ version = "{framework["version"]}", path = "{dependency}", features = ["claw", "auth", "view"] }}',
            generated_text,
            flags=re.MULTILINE,
        )
        if replacements != 1:
            raise RuntimeError("expected exactly one generated Berserk dependency")
        manifest.write_text(manifest_text)
        generated_main = (consumer / "src/main.rs").read_text()
        if "app::routes::register(&mut app)?" not in generated_main:
            raise RuntimeError("application codegen lost route registration")
        if "app.middleware(RequestId)" not in generated_main:
            raise RuntimeError("application codegen lost request-id middleware")

        generated_config = (consumer / "src/config/mod.rs").read_text()
        if 'value("BERSERK_ADDRESS", "127.0.0.1:3000")?' not in generated_config:
            raise RuntimeError("application codegen lost the default listen address")
        if 'value("APP_NAME", "Berserk API")?' not in generated_config:
            raise RuntimeError("application codegen lost the default application name")

        generated_routes = (consumer / "src/app/routes.rs").read_text()
        if 'app.route().get("/health"' not in generated_routes:
            raise RuntimeError("generated application routes are not using route codegen")
        if "request.config::<AppConfig>()" not in generated_routes:
            raise RuntimeError("generated application routes lost the welcome route contract")
        for kind, name in [
            ("model", "User"),
            ("controller", "UserController"),
            ("middleware", "Audit"),
            ("resource", "UserResource"),
            ("policy", "UserPolicy"),
        ]:
            run(str(cli), f"make:{kind}", name, cwd=consumer)
        run(str(cli), "make:request", "SimpleInput", cwd=consumer)
        run(str(cli), "make:request", "CreateUser", "--model", "User", cwd=consumer)
        run(str(cli), "make:migration", "create_users_table", cwd=consumer)
        run(str(cli), "make:crud", "Post", cwd=consumer)
        run(
            str(cli),
            "make:controller",
            "UserResourceController",
            "--resource",
            "--model",
            "User",
            "--request",
            "CreateUser",
            cwd=consumer,
        )
        (consumer / "src/lib.rs").write_text('''pub mod app;
pub mod config;
pub mod database;
#[cfg(test)] mod tests {
    #[test] fn generated_form_runs_through_real_controller_adapter() {
        use berserk::{App, Headers, Method, Request, Response};
        let mut app = App::new();
        app.route().post("/users", |input: crate::app::validations::create_user::CreateUser| Response::text(input.name)).unwrap();
        let mut headers = Headers::new();
        headers.insert("content-type", "application/json").unwrap();
        let response = app.respond(Request::new(Method::new("POST").unwrap(), "/users", headers, br#"{"name":" Ada "}"#.to_vec()).unwrap());
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.body(), b"Ada");
    }
}
''')
        generated_post_model = (consumer / "src/app/models/post.rs").read_text()
        if "#[fillable]" not in generated_post_model or "pub name: String" not in generated_post_model:
            raise RuntimeError("CRUD module model is not aligned with its generated request")

        generated_post_request = (consumer / "src/app/validations/post_input.rs").read_text()
        if "impl IntoInsert<Post> for PostInput" not in generated_post_request:
            raise RuntimeError("CRUD module request is not model-bound")

        generated_post_controller = (
            consumer / "src/app/controllers/post_controller.rs"
        ).read_text()
        if "impl CrudController for PostController" not in generated_post_controller:
            raise RuntimeError("CRUD module controller is not using the CRUD contract")

        generated_routes = (consumer / "src/app/routes.rs").read_text()
        if 'app.route().crud("/posts", PostController)?;' not in generated_routes:
            raise RuntimeError("CRUD module route was not registered")

        post_migrations = list(
            (consumer / "src/database/migrations").glob("*_create_posts_table.rs")
        )
        if len(post_migrations) != 1:
            raise RuntimeError("expected exactly one CRUD module migration")
        if 'Column::string("name")' not in post_migrations[0].read_text():
            raise RuntimeError("CRUD module migration is not aligned with its model")

        migration_index = (consumer / "src/database/migrations/mod.rs").read_text()
        if "create_posts_table_" not in migration_index:
            raise RuntimeError("CRUD module migration is not compiled through its module index")

        generated_middleware = (consumer / "src/app/middleware/audit.rs").read_text()
        if "impl Middleware for Audit" not in generated_middleware:
            raise RuntimeError("middleware scaffold did not come from the codegen contract")

        generated_resource = (consumer / "src/app/resources/user_resource.rs").read_text()
        if "impl ApiResource for UserResource" not in generated_resource:
            raise RuntimeError("resource scaffold did not come from the codegen contract")

        generated_policy = (consumer / "src/app/policies/user_policy.rs").read_text()
        if "impl Policy<u64> for UserPolicy" not in generated_policy:
            raise RuntimeError("policy scaffold did not come from the codegen contract")
        if "Decision::Deny" not in generated_policy:
            raise RuntimeError("generated policy must deny by default")

        generated_model = (consumer / "src/app/models/user.rs").read_text()
        if '#[table("users")]' not in generated_model:
            raise RuntimeError("model codegen did not generate the conventional users table")
        if "#[primary_key]" not in generated_model:
            raise RuntimeError("model codegen did not generate the conventional primary key")

        migrations = list((consumer / "src/database/migrations").glob("*_create_users_table.rs"))
        if len(migrations) != 1:
            raise RuntimeError("expected exactly one generated create_users_table migration")
        generated_migration = migrations[0].read_text()
        if 'Table::create("users")' not in generated_migration:
            raise RuntimeError("create-table migration did not generate the users table plan")
        if 'Table::drop("users")' not in generated_migration:
            raise RuntimeError("create-table migration did not generate the rollback plan")

        generated_request = (consumer / "src/app/validations/create_user.rs").read_text()
        if "impl IntoInsert<User> for CreateUser" not in generated_request:
            raise RuntimeError("model-bound request did not generate an insert mapping")
        if "impl IntoUpdate<User> for CreateUser" not in generated_request:
            raise RuntimeError("model-bound request did not generate an update mapping")
        generated_controller = (
            consumer / "src/app/controllers/user_resource_controller.rs"
        ).read_text()
        if "impl CrudController for UserResourceController" not in generated_controller:
            raise RuntimeError("resource controller was not generated through the CRUD contract")
        run("cargo", "test", "--manifest-path", str(manifest), cwd=consumer)

if __name__ == "__main__":
    main()
