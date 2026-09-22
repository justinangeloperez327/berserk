use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=app/views");

    let root = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR"),
    )
    .join("app/views");

    if let Err(error) = berserk_axe::validate_views_from(root) {
        panic!("Axe view validation failed: {error}");
    }
}
