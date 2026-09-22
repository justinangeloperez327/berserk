use berserk_axe::{render_from, CompiledViews, Context, Error};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn test_root(name: &str) -> PathBuf {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "berserk-axe-compiled-{name}-{}-{id}",
        std::process::id()
    ))
}

fn write(root: &std::path::Path, relative: &str, source: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, source).unwrap();
}

#[test]
fn compiled_views_expand_includes_and_keep_deterministic_metadata() {
    let root = test_root("includes");
    write(
        &root,
        "pages/index.html",
        "@include(\"shared/header\")\n<main>{{ body }}</main>",
    );
    write(&root, "shared/header.html", "<header>{{ title }}</header>");

    let views = CompiledViews::compile(&root).unwrap();
    assert_eq!(
        views.views().collect::<Vec<_>>(),
        ["pages/index", "shared/header"]
    );
    assert_eq!(
        views.dependencies("pages/index").unwrap(),
        ["shared/header".to_owned()]
    );

    let rendered = views
        .render(
            "pages/index",
            &Context::new().with("title", "<Axe>").with("body", "Ready"),
        )
        .unwrap();
    let expected = "<header>&lt;Axe&gt;</header>\n<main>Ready</main>";
    assert_eq!(rendered, expected);

    let rendered_from_root = render_from(
        &root,
        "pages/index",
        &Context::new().with("title", "<Axe>").with("body", "Ready"),
    )
    .unwrap();
    assert_eq!(rendered_from_root, expected);

    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        views
            .render(
                "pages/index",
                &Context::new().with("title", "<Axe>").with("body", "Ready"),
            )
            .unwrap(),
        expected
    );
}

#[test]
fn missing_include_reports_parent_file_line_and_column() {
    let root = test_root("missing-include");
    write(
        &root,
        "pages/index.html",
        "<h1>Users</h1>\n  @include(\"shared/missing\")",
    );

    let error = CompiledViews::compile(&root).unwrap_err();
    assert!(matches!(
        error,
        Error::ViewCompile {
            view,
            line: 2,
            column: 3,
            message,
        } if view == "pages/index"
            && message.contains("shared/missing")
            && message.contains("not found")
    ));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn malformed_template_reports_the_actual_view_location() {
    let root = test_root("diagnostic");
    write(
        &root,
        "pages/index.html",
        "@include(\"shared/header\")\n<main>ok</main>",
    );
    write(
        &root,
        "shared/header.html",
        "<header>\n{{ bad path }}\n</header>",
    );

    let error = CompiledViews::compile(&root).unwrap_err();
    assert!(matches!(
        error,
        Error::ViewCompile {
            view,
            line: 2,
            column: 1,
            message,
        } if view == "shared/header" && message.contains("bad path")
    ));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn include_cycles_are_rejected_with_dependency_context() {
    let root = test_root("cycle");
    write(&root, "a.html", "@include(\"b\")");
    write(&root, "b.html", "<p>B</p>\n@include(\"a\")");

    let error = CompiledViews::compile(&root).unwrap_err();
    assert!(matches!(
        error,
        Error::ViewCompile {
            view,
            line: 2,
            column: 1,
            message,
        } if view == "b" && message.contains("a -> b -> a")
    ));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn include_targets_cannot_escape_the_view_root() {
    let root = test_root("escape");
    write(&root, "index.html", "@include(\"../secret\")");

    let error = CompiledViews::compile(&root).unwrap_err();
    assert!(matches!(
        error,
        Error::ViewCompile {
            view,
            line: 1,
            column: 1,
            message,
        } if view == "index" && message.contains("invalid @include target")
    ));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn foundation_view_tree_passes_the_production_compiler() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/foundation/app/views");
    let views = CompiledViews::compile(root).unwrap();
    assert!(views.contains("users/index"));
}
