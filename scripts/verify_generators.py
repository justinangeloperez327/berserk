#!/usr/bin/env python3
"""Compile every application generator in an independent consumer project."""
from pathlib import Path
import os
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

def run(*args, cwd=ROOT):
    subprocess.run(args, cwd=cwd, check=True)


def main():
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
        dependency = (ROOT / "crates/framework").as_posix()
        manifest_text, replacements = re.subn(
            r"^berserk\s*=\s*.+$",
            lambda _: f'berserk = {{ path = "{dependency}", features = ["claw", "auth"] }}',
            manifest.read_text(),
            flags=re.MULTILINE,
        )
        if replacements != 1:
            raise RuntimeError("expected exactly one generated Berserk dependency")
        manifest.write_text(manifest_text)
        for kind, name in [("model", "User"), ("controller", "UserController"), ("request", "CreateUser"), ("resource", "UserResource"), ("policy", "UserPolicy")]:
            run(str(cli), f"make:{kind}", name, cwd=consumer)
        (consumer / "src/lib.rs").write_text('''pub mod models;
pub mod controllers;
pub mod requests;
pub mod resources;
pub mod policies;
#[cfg(test)] mod tests {
    #[test] fn generated_form_runs_through_real_controller_adapter() {
        use berserk::{App, Headers, Method, Request, Response};
        let mut app = App::new();
        app.route().post("/users", |input: crate::requests::create_user::CreateUser| Response::text(input.name)).unwrap();
        let mut headers = Headers::new();
        headers.insert("content-type", "application/json").unwrap();
        let response = app.respond(Request::new(Method::new("POST").unwrap(), "/users", headers, br#"{"name":" Ada "}"#.to_vec()).unwrap());
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.body(), b"Ada");
    }
}
''')
        run("cargo", "test", "--manifest-path", str(manifest), cwd=consumer)

if __name__ == "__main__":
    main()
