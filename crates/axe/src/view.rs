use crate::{Context, Error, Result, Template};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, RwLock},
};

const MAX_INCLUDE_DEPTH: usize = 32;
const INCLUDE_MARKER: &str = "@include(";

static CACHE: OnceLock<RwLock<HashMap<PathBuf, Arc<CompiledViews>>>> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct CompiledViews {
    templates: BTreeMap<String, Template>,
    dependencies: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug)]
struct SourceView {
    source: String,
    includes: Vec<Include>,
}

#[derive(Clone, Debug)]
struct Include {
    start: usize,
    end: usize,
    target: String,
}

impl CompiledViews {
    /// Read, validate, resolve dependencies, and compile every .html view under a root.
    ///
    /// Directory traversal and view ordering are deterministic. Includes are resolved
    /// before the final template is compiled, so a successful value can be reused
    /// without reading template files while rendering.
    pub fn compile(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let mut raw = BTreeMap::new();
        collect_views(root, root, &mut raw)?;

        let mut sources = BTreeMap::new();
        let mut dependencies = BTreeMap::new();
        for (view, source) in raw {
            let includes = parse_includes(&view, &source)?;
            let masked = mask_includes(&source, &includes);
            Template::compile_named(&view, &masked)?;
            dependencies.insert(
                view.clone(),
                includes.iter().map(|include| include.target.clone()).collect(),
            );
            sources.insert(view, SourceView { source, includes });
        }

        let mut expanded = BTreeMap::new();
        let names: Vec<_> = sources.keys().cloned().collect();
        for view in &names {
            let mut stack = Vec::new();
            let _ = expand_view(view, &sources, &mut stack, &mut expanded)?;
        }

        let mut templates = BTreeMap::new();
        for view in names {
            let source = expanded
                .get(&view)
                .expect("every discovered view is expanded before compilation");
            templates.insert(view.clone(), Template::compile_named(&view, source)?);
        }

        Ok(Self {
            templates,
            dependencies,
        })
    }

    pub fn render(&self, view: &str, context: &Context) -> Result<String> {
        validate_view_name(view)?;
        self.templates
            .get(view)
            .ok_or_else(|| Error::ViewNotFound(view.to_owned()))?
            .render(context)
    }

    pub fn contains(&self, view: &str) -> bool {
        self.templates.contains_key(view)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    pub fn views(&self) -> impl Iterator<Item = &str> {
        self.templates.keys().map(String::as_str)
    }

    /// Direct include dependencies declared by a view, in source order.
    pub fn dependencies(&self, view: &str) -> Option<&[String]> {
        self.dependencies.get(view).map(Vec::as_slice)
    }
}

pub fn render(view: &str, context: &Context) -> Result<String> {
    render_from("app/views", view, context)
}

pub fn render_from(root: impl AsRef<Path>, view: &str, context: &Context) -> Result<String> {
    let root = root.as_ref();
    let path = view_path(root, view)?;
    if !path.is_file() {
        return Err(Error::ViewNotFound(view.to_owned()));
    }

    if cfg!(debug_assertions) {
        return CompiledViews::compile(root)?.render(view, context);
    }

    production_views(root)?.render(view, context)
}

/// Validate the conventional application view root without rendering.
pub fn validate_views() -> Result<()> {
    validate_views_from("app/views")
}

/// Validate and precompile a view tree. This is suitable for build.rs or CI gates.
pub fn validate_views_from(root: impl AsRef<Path>) -> Result<()> {
    CompiledViews::compile(root).map(|_| ())
}

fn production_views(root: &Path) -> Result<Arc<CompiledViews>> {
    let cache = CACHE.get_or_init(|| RwLock::new(HashMap::new()));

    if let Some(views) = cache
        .read()
        .map_err(|_| Error::ViewCache)?
        .get(root)
        .cloned()
    {
        return Ok(views);
    }

    let compiled = Arc::new(CompiledViews::compile(root)?);
    let mut cache = cache.write().map_err(|_| Error::ViewCache)?;
    Ok(cache
        .entry(root.to_path_buf())
        .or_insert_with(|| compiled.clone())
        .clone())
}

fn collect_views(
    root: &Path,
    directory: &Path,
    views: &mut BTreeMap<String, String>,
) -> Result<()> {
    let mut entries = std::fs::read_dir(directory)
        .map_err(|_| Error::ReadView(directory.display().to_string()))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|_| Error::ReadView(directory.display().to_string()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let file_type = entry
            .file_type()
            .map_err(|_| Error::ReadView(entry.path().display().to_string()))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_views(root, &entry.path(), views)?;
            continue;
        }
        if !file_type.is_file() || entry.path().extension().and_then(|value| value.to_str()) != Some("html") {
            continue;
        }

        let path = entry.path();
        let view = view_name(root, &path)?;
        let source =
            std::fs::read_to_string(&path).map_err(|_| Error::ReadView(view.clone()))?;
        views.insert(view, source);
    }
    Ok(())
}

fn view_name(root: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| Error::InvalidViewName(path.display().to_string()))?
        .with_extension("");
    let mut segments = Vec::new();
    for component in relative.components() {
        let segment = component
            .as_os_str()
            .to_str()
            .ok_or_else(|| Error::InvalidViewName(relative.display().to_string()))?;
        validate_view_segment(segment)?;
        segments.push(segment);
    }
    let view = segments.join("/");
    validate_view_name(&view)?;
    Ok(view)
}

fn parse_includes(view: &str, source: &str) -> Result<Vec<Include>> {
    let bytes = source.as_bytes();
    let mut includes = Vec::new();
    let mut cursor = 0;

    while let Some(relative) = source[cursor..].find(INCLUDE_MARKER) {
        let start = cursor + relative;
        let mut position = start + INCLUDE_MARKER.len();
        skip_ascii_whitespace(bytes, &mut position);

        if bytes.get(position) != Some(&b'"') {
            return Err(diagnostic(
                view,
                source,
                start,
                "expected a double-quoted view name in @include",
            ));
        }
        position += 1;
        let target_start = position;
        while let Some(byte) = bytes.get(position) {
            if *byte == b'"' {
                break;
            }
            position += 1;
        }
        if bytes.get(position) != Some(&b'"') {
            return Err(diagnostic(
                view,
                source,
                start,
                "unclosed view name in @include",
            ));
        }

        let target = &source[target_start..position];
        validate_view_name(target).map_err(|error| {
            diagnostic(
                view,
                source,
                start,
                format!("invalid @include target: {error}"),
            )
        })?;

        position += 1;
        skip_ascii_whitespace(bytes, &mut position);
        if bytes.get(position) != Some(&b')') {
            return Err(diagnostic(
                view,
                source,
                start,
                "expected ')' after @include view name",
            ));
        }
        position += 1;

        includes.push(Include {
            start,
            end: position,
            target: target.to_owned(),
        });
        cursor = position;
    }

    Ok(includes)
}

fn mask_includes(source: &str, includes: &[Include]) -> String {
    let mut masked = source.to_owned();
    for include in includes.iter().rev() {
        let replacement: String = source[include.start..include.end]
            .bytes()
            .map(|byte| if byte == b'\n' { '\n' } else { ' ' })
            .collect();
        masked.replace_range(include.start..include.end, &replacement);
    }
    masked
}

fn expand_view(
    view: &str,
    sources: &BTreeMap<String, SourceView>,
    stack: &mut Vec<String>,
    memo: &mut BTreeMap<String, String>,
) -> Result<String> {
    if let Some(source) = memo.get(view) {
        return Ok(source.clone());
    }

    let current = sources
        .get(view)
        .ok_or_else(|| Error::ViewNotFound(view.to_owned()))?;
    stack.push(view.to_owned());

    let mut output = String::with_capacity(current.source.len());
    let mut cursor = 0;
    for include in &current.includes {
        output.push_str(&current.source[cursor..include.start]);

        if stack.len() >= MAX_INCLUDE_DEPTH {
            stack.pop();
            return Err(diagnostic(
                view,
                &current.source,
                include.start,
                format!("view include depth exceeds {MAX_INCLUDE_DEPTH}"),
            ));
        }
        if let Some(index) = stack.iter().position(|candidate| candidate == &include.target) {
            let mut cycle = stack[index..].to_vec();
            cycle.push(include.target.clone());
            stack.pop();
            return Err(diagnostic(
                view,
                &current.source,
                include.start,
                format!("view include cycle detected: {}", cycle.join(" -> ")),
            ));
        }
        if !sources.contains_key(&include.target) {
            stack.pop();
            return Err(diagnostic(
                view,
                &current.source,
                include.start,
                format!("included view '{}' was not found", include.target),
            ));
        }

        output.push_str(&expand_view(&include.target, sources, stack, memo)?);
        cursor = include.end;
    }
    output.push_str(&current.source[cursor..]);
    stack.pop();

    memo.insert(view.to_owned(), output.clone());
    Ok(output)
}

fn skip_ascii_whitespace(bytes: &[u8], position: &mut usize) {
    while matches!(
        bytes.get(*position),
        Some(b' ' | b'\t' | b'\r' | b'\n')
    ) {
        *position += 1;
    }
}

fn diagnostic(
    view: &str,
    source: &str,
    position: usize,
    message: impl Into<String>,
) -> Error {
    let (line, column) = line_column(source, position);
    Error::ViewCompile {
        view: view.to_owned(),
        line,
        column,
        message: message.into(),
    }
}

fn line_column(source: &str, position: usize) -> (usize, usize) {
    let position = position.min(source.len());
    let prefix = &source[..position];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or_else(|| prefix.chars().count() + 1, |(_, tail)| tail.chars().count() + 1);
    (line, column)
}

fn view_path(root: &Path, view: &str) -> Result<PathBuf> {
    validate_view_name(view)?;

    let mut path = root.to_path_buf();
    for segment in view.split('/') {
        path.push(segment);
    }
    path.set_extension("html");
    Ok(path)
}

fn validate_view_name(view: &str) -> Result<()> {
    if view.is_empty() || view.starts_with('/') || view.contains('\\') {
        return Err(Error::InvalidViewName(view.to_owned()));
    }
    for segment in view.split('/') {
        validate_view_segment(segment)?;
    }
    Ok(())
}

fn validate_view_segment(segment: &str) -> Result<()> {
    if segment.is_empty()
        || segment == "."
        || segment == ".."
        || !segment
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(Error::InvalidViewName(segment.to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("berserk-axe-{name}-{}", std::process::id()))
    }

    #[test]
    fn named_view_resolves_under_root() {
        let root = test_root("resolve");
        let path = root.join("users/index.html");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "<h1>{{ title }}</h1>").unwrap();

        let rendered = render_from(
            &root,
            "users/index",
            &Context::new().with("title", "<Users>"),
        )
        .unwrap();

        assert_eq!(rendered, "<h1>&lt;Users&gt;</h1>");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn view_names_cannot_escape_root() {
        let root = test_root("invalid");
        let error = render_from(&root, "../secret", &Context::new()).unwrap_err();
        assert_eq!(error, Error::InvalidViewName("../secret".into()));
    }

    #[test]
    fn missing_named_view_is_explicit() {
        let root = test_root("missing");
        let error = render_from(&root, "users/missing", &Context::new()).unwrap_err();
        assert_eq!(error, Error::ViewNotFound("users/missing".into()));
    }
}
