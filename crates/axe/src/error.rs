use std::{error::Error as StdError, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    EmptyExpression,
    InvalidExpression(String),
    InvalidDirective(String),
    UnexpectedDirective(String),
    UnclosedExpression,
    UnclosedDirective(&'static str),
    MissingValue(String),
    ExpectedList(String),
    UnsafeRaw(String),
    UnsupportedValue(String),
    InvalidViewName(String),
    ViewNotFound(String),
    ReadView(String),
    ViewCompile {
        view: String,
        line: usize,
        column: usize,
        message: String,
    },
    ViewCache,
    Write,
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyExpression => f.write_str("view expression cannot be empty"),
            Self::InvalidExpression(expression) => {
                write!(f, "invalid view expression `{expression}`")
            }
            Self::InvalidDirective(directive) => {
                write!(f, "invalid view directive `{directive}`")
            }
            Self::UnexpectedDirective(directive) => {
                write!(f, "unexpected view directive `{directive}`")
            }
            Self::UnclosedExpression => f.write_str("unclosed view expression"),
            Self::UnclosedDirective(directive) => {
                write!(f, "unclosed view directive `{directive}`")
            }
            Self::MissingValue(path) => write!(f, "view value `{path}` was not provided"),
            Self::ExpectedList(path) => write!(f, "view value `{path}` is not a list"),
            Self::UnsafeRaw(path) => write!(
                f,
                "raw view output `{path}` requires SafeHtml; ordinary text is escaped"
            ),
            Self::UnsupportedValue(path) => {
                write!(f, "view value `{path}` cannot be rendered directly")
            }
            Self::InvalidViewName(view) => write!(f, "invalid view name `{view}`"),
            Self::ViewNotFound(view) => write!(f, "view `{view}` was not found"),
            Self::ReadView(view) => write!(f, "failed to read view `{view}`"),
            Self::ViewCompile {
                view,
                line,
                column,
                message,
            } => write!(f, "{view}:{line}:{column}: {message}"),
            Self::ViewCache => f.write_str("view cache lock failed"),
            Self::Write => f.write_str("failed to write rendered view output"),
        }
    }
}

impl StdError for Error {}

impl From<fmt::Error> for Error {
    fn from(_: fmt::Error) -> Self {
        Self::Write
    }
}
