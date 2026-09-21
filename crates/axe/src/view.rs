use crate::{Context, Error, Result, Template};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
};

static CACHE: OnceLock<RwLock<HashMap<PathBuf, Template>>> = OnceLock::new();

pub fn render(view: &str, context: &Context) -> Result<String> {
    render_from("app/views", view, context)
}

pub fn render_from(root: impl AsRef<Path>, view: &str, context: &Context) -> Result<String> {
    let path = view_path(root.as_ref(), view)?;
    let template = load_template(&path, view)?;
    template.render(context)
}

fn load_template(path: &Path, view: &str) -> Result<Template> {
    if cfg!(debug_assertions) {
        return compile_file(path, view);
    }

    let cache = CACHE.get_or_init(|| RwLock::new(HashMap::new()));

    if let Some(template) = cache
        .read()
        .map_err(|_| Error::ViewCache)?
        .get(path)
        .cloned()
    {
        return Ok(template);
    }

    let template = compile_file(path, view)?;
    cache
        .write()
        .map_err(|_| Error::ViewCache)?
        .insert(path.to_owned(), template.clone());
    Ok(template)
}

fn compile_file(path: &Path, view: &str) -> Result<Template> {
    let source = std::fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Error::ViewNotFound(view.to_owned())
        } else {
            Error::ReadView(view.to_owned())
        }
    })?;
    Template::compile(source)
}

fn view_path(root: &Path, view: &str) -> Result<PathBuf> {
    if view.is_empty() || view.starts_with('/') || view.contains('\\') {
        return Err(Error::InvalidViewName(view.to_owned()));
    }

    let mut path = root.to_path_buf();
    for segment in view.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || !segment.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
            })
        {
            return Err(Error::InvalidViewName(view.to_owned()));
        }
        path.push(segment);
    }
    path.set_extension("html");
    Ok(path)
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
