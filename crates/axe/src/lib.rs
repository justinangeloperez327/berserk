//! Axe is Berserk's HTML-first view engine.
//!
//! Templates remain ordinary HTML with a deliberately small set of dynamic
//! directives. `Template::compile` parses a template once; rendering reuses the
//! compiled representation without reparsing the source.
#![forbid(unsafe_code)]

mod context;
mod error;
mod escape;
mod template;
mod value;
mod view;

pub use context::Context;
pub use error::{Error, Result};
pub use template::Template;
pub use value::{SafeHtml, Value};
pub use view::{render, render_from};

#[macro_export]
macro_rules! view_data {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut context = $crate::Context::new();
        $(
            let _ = context.insert($key, $value);
        )*
        context
    }};
}

#[macro_export]
macro_rules! view_object {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut values = ::std::collections::BTreeMap::<String, $crate::Value>::new();
        $(
            let _ = values.insert(($key).into(), ($value).into());
        )*
        $crate::Value::Object(values)
    }};
}
