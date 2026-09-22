# Axe views

Axe is Berserk's HTML-first view engine. Named application views live under
`app/views` by default and use `.html` files. View names are root-relative
paths without the extension, for example `users/index`.

## Template syntax

Axe keeps the template surface deliberately small:

```html
<h1>{{ title }}</h1>

@if(show_users)
    <ul>
    @foreach(user in users)
        <li>{{ user.name }}</li>
    @endforeach
    </ul>
@else
    <p>No users</p>
@endif
```

`{{ value }}` HTML-escapes output. `{!! value !!}` accepts only
`SafeHtml`; ordinary strings cannot bypass escaping.

## Includes

Named view trees support explicit root-relative includes:

```html
@include("shared/header")
<main>{{ content }}</main>
@include("shared/footer")
```

An include target uses the same view-name rules as `view(...)`: path segments
may contain ASCII letters, digits, `_`, and `-`. Absolute paths, `.`,
`..`, backslashes, dynamic include names, and file extensions are rejected.

Includes are resolved before the final template is compiled. Missing
dependencies and dependency cycles fail compilation. Include chains are bounded
to 32 views so a malformed dependency graph cannot recurse without limit.

## Production compilation

`CompiledViews::compile(root)` walks the complete view tree in deterministic
name order, validates every template, resolves includes, detects missing/cyclic
dependencies, and produces reusable compiled templates.

In debug builds, `render_from` recompiles the view tree so template edits are
visible without restarting the process. In release builds, Axe compiles the
complete root once and caches that compiled set for subsequent renders.

Production deployments should also validate views during the application build
so template failures are caught before a process receives traffic. The
foundation application uses this pattern:

```toml
[build-dependencies]
berserk-axe = "1.0"
```

```rust
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
```

Applications using the conventional root can call `validate_views()` instead.
The validation step parses the same templates and dependency graph used by the
runtime compiler; it does not maintain a second syntax implementation.

## Diagnostics

Named-view compilation errors include the originating view and source location:

```text
shared/header:2:1: invalid view expression `bad path`
```

Malformed include syntax, missing include targets, cycles, and ordinary Axe
parser failures use the same file/line/column diagnostic form. Standalone
`Template::compile` retains its existing source-only error API.

## Filesystem behavior

Only regular `.html` files under the selected root are compiled. Symlinked
files and directories are not followed. Non-HTML files are ignored. View names
cannot escape the configured root.

For model and collection conversion into Axe values, see
[model presentation](model-presentation.md).
