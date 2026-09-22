use crate::{escape, Context, Error, Result, Value};
use std::fmt::Write;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Template {
    nodes: Vec<Node>,
    capacity_hint: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Node {
    Text(String),
    Echo {
        path: String,
        raw: bool,
    },
    If {
        condition: Condition,
        then_nodes: Vec<Node>,
        else_nodes: Vec<Node>,
    },
    ForEach {
        binding: String,
        collection: String,
        nodes: Vec<Node>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Condition {
    path: String,
    negated: bool,
}

impl Template {
    pub fn compile(source: impl AsRef<str>) -> Result<Self> {
        Self::compile_inner(source.as_ref()).map_err(|failure| failure.error)
    }

    pub(crate) fn compile_named(view: &str, source: &str) -> Result<Self> {
        Self::compile_inner(source).map_err(|failure| {
            let (line, column) = line_column(source, failure.position);
            Error::ViewCompile {
                view: view.to_owned(),
                line,
                column,
                message: failure.error.to_string(),
            }
        })
    }

    fn compile_inner(source: &str) -> ParseResult<Self> {
        let mut parser = Parser::new(source);
        let (nodes, stop) = parser.parse_nodes(&[])?;
        if let Some(stop) = stop {
            return Err(parser.failure(Error::UnexpectedDirective(stop.to_owned())));
        }
        Ok(Self {
            nodes,
            capacity_hint: source.len(),
        })
    }

    pub fn render(&self, context: &Context) -> Result<String> {
        let mut output = String::with_capacity(self.capacity_hint);
        self.render_into(context, &mut output)?;
        Ok(output)
    }

    pub fn render_into(&self, context: &Context, output: &mut impl Write) -> Result<()> {
        let mut locals = Vec::new();
        render_nodes(&self.nodes, context, &mut locals, output)
    }
}

fn render_nodes<'node, 'value>(
    nodes: &'node [Node],
    context: &'value Context,
    locals: &mut Vec<(&'node str, &'value Value)>,
    output: &mut impl Write,
) -> Result<()> {
    for node in nodes {
        match node {
            Node::Text(text) => output.write_str(text)?,
            Node::Echo { path, raw } => {
                let value = resolve(context, locals, path)
                    .ok_or_else(|| Error::MissingValue(path.clone()))?;
                write_value(output, path, value, *raw)?;
            }
            Node::If {
                condition,
                then_nodes,
                else_nodes,
            } => {
                let value = resolve(context, locals, &condition.path)
                    .ok_or_else(|| Error::MissingValue(condition.path.clone()))?;
                let truthy = if condition.negated {
                    !value.is_truthy()
                } else {
                    value.is_truthy()
                };
                let branch = if truthy { then_nodes } else { else_nodes };
                render_nodes(branch, context, locals, output)?;
            }
            Node::ForEach {
                binding,
                collection,
                nodes,
            } => {
                let values = resolve(context, locals, collection)
                    .ok_or_else(|| Error::MissingValue(collection.clone()))?;
                let Value::List(values) = values else {
                    return Err(Error::ExpectedList(collection.clone()));
                };
                for value in values {
                    locals.push((binding, value));
                    let result = render_nodes(nodes, context, locals, output);
                    locals.pop();
                    result?;
                }
            }
        }
    }
    Ok(())
}

fn resolve<'value>(
    context: &'value Context,
    locals: &[(&str, &'value Value)],
    path: &str,
) -> Option<&'value Value> {
    let mut parts = path.split('.');
    let first = parts.next()?;
    let mut value = locals
        .iter()
        .rev()
        .find_map(|(name, value)| (*name == first).then_some(*value))
        .or_else(|| context.get(first))?;
    for part in parts {
        value = value.field(part)?;
    }
    Some(value)
}

fn write_value(output: &mut impl Write, path: &str, value: &Value, raw: bool) -> Result<()> {
    if raw {
        return match value {
            Value::SafeHtml(value) => output.write_str(value.as_str()).map_err(Into::into),
            _ => Err(Error::UnsafeRaw(path.to_owned())),
        };
    }

    match value {
        Value::Null => Ok(()),
        Value::Bool(value) => write!(output, "{value}").map_err(Into::into),
        Value::Number(value) | Value::Text(value) => {
            escape::html_into(output, value).map_err(Into::into)
        }
        Value::SafeHtml(value) => escape::html_into(output, value.as_str()).map_err(Into::into),
        Value::Bytes(_) | Value::List(_) | Value::Object(_) => {
            Err(Error::UnsupportedValue(path.to_owned()))
        }
    }
}

#[derive(Debug)]
struct ParseFailure {
    error: Error,
    position: usize,
}

type ParseResult<T> = std::result::Result<T, ParseFailure>;

struct Parser<'source> {
    source: &'source str,
    position: usize,
}

impl<'source> Parser<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    fn parse_nodes(
        &mut self,
        stops: &[&'static str],
    ) -> ParseResult<(Vec<Node>, Option<&'static str>)> {
        let mut nodes = Vec::new();
        while self.position < self.source.len() {
            if let Some(stop) = stops
                .iter()
                .copied()
                .find(|stop| self.remaining().starts_with(*stop))
            {
                return Ok((nodes, Some(stop)));
            }

            let Some((offset, marker)) = self.next_marker() else {
                nodes.push(Node::Text(self.remaining().to_owned()));
                self.position = self.source.len();
                break;
            };

            if offset > 0 {
                let end = self.position + offset;
                nodes.push(Node::Text(self.source[self.position..end].to_owned()));
                self.position = end;
                continue;
            }

            match marker {
                "{{" => nodes.push(self.parse_echo(false)?),
                "{!!" => nodes.push(self.parse_echo(true)?),
                "@if(" => nodes.push(self.parse_if()?),
                "@foreach(" => nodes.push(self.parse_foreach()?),
                "@else" | "@endif" | "@endforeach" => {
                    return Err(self.failure(Error::UnexpectedDirective(marker.to_owned())));
                }
                _ => unreachable!("known marker"),
            }
        }
        Ok((nodes, None))
    }

    fn parse_echo(&mut self, raw: bool) -> ParseResult<Node> {
        let start = self.position;
        let (open, close) = if raw { ("{!!", "!!}") } else { ("{{", "}}") };
        self.position += open.len();
        let rest = self.remaining();
        let end = rest.find(close).ok_or_else(|| ParseFailure {
            error: Error::UnclosedExpression,
            position: start,
        })?;
        let expression = rest[..end].trim();
        validate_path(expression).map_err(|error| ParseFailure {
            error,
            position: start,
        })?;
        self.position += end + close.len();
        Ok(Node::Echo {
            path: expression.to_owned(),
            raw,
        })
    }

    fn parse_if(&mut self) -> ParseResult<Node> {
        let start = self.position;
        self.position += "@if(".len();
        let end = self
            .remaining()
            .find(')')
            .ok_or_else(|| ParseFailure {
                error: Error::UnclosedDirective("@if"),
                position: start,
            })?;
        let expression = self.remaining()[..end].trim();
        let condition = parse_condition(expression).map_err(|error| ParseFailure {
            error,
            position: start,
        })?;
        self.position += end + 1;

        let (then_nodes, stop) = self.parse_nodes(&["@else", "@endif"])?;
        match stop {
            Some("@endif") => {
                self.position += "@endif".len();
                Ok(Node::If {
                    condition,
                    then_nodes,
                    else_nodes: Vec::new(),
                })
            }
            Some("@else") => {
                self.position += "@else".len();
                let (else_nodes, stop) = self.parse_nodes(&["@endif"])?;
                if stop != Some("@endif") {
                    return Err(ParseFailure {
                        error: Error::UnclosedDirective("@if"),
                        position: start,
                    });
                }
                self.position += "@endif".len();
                Ok(Node::If {
                    condition,
                    then_nodes,
                    else_nodes,
                })
            }
            _ => Err(ParseFailure {
                error: Error::UnclosedDirective("@if"),
                position: start,
            }),
        }
    }

    fn parse_foreach(&mut self) -> ParseResult<Node> {
        let start = self.position;
        self.position += "@foreach(".len();
        let end = self
            .remaining()
            .find(')')
            .ok_or_else(|| ParseFailure {
                error: Error::UnclosedDirective("@foreach"),
                position: start,
            })?;
        let expression = self.remaining()[..end].trim();
        let Some((binding, collection)) = expression.split_once(" in ") else {
            return Err(ParseFailure {
                error: Error::InvalidDirective(format!("@foreach({expression})")),
                position: start,
            });
        };
        let binding = binding.trim();
        let collection = collection.trim();
        validate_identifier(binding).map_err(|error| ParseFailure {
            error,
            position: start,
        })?;
        validate_path(collection).map_err(|error| ParseFailure {
            error,
            position: start,
        })?;
        self.position += end + 1;

        let (nodes, stop) = self.parse_nodes(&["@endforeach"])?;
        if stop != Some("@endforeach") {
            return Err(ParseFailure {
                error: Error::UnclosedDirective("@foreach"),
                position: start,
            });
        }
        self.position += "@endforeach".len();
        Ok(Node::ForEach {
            binding: binding.to_owned(),
            collection: collection.to_owned(),
            nodes,
        })
    }

    fn next_marker(&self) -> Option<(usize, &'static str)> {
        const MARKERS: [&str; 7] = [
            "{{",
            "{!!",
            "@if(",
            "@else",
            "@endif",
            "@foreach(",
            "@endforeach",
        ];
        MARKERS
            .iter()
            .filter_map(|marker| self.remaining().find(*marker).map(|index| (index, *marker)))
            .min_by_key(|(index, _)| *index)
    }

    fn remaining(&self) -> &'source str {
        &self.source[self.position..]
    }

    fn failure(&self, error: Error) -> ParseFailure {
        ParseFailure {
            error,
            position: self.position,
        }
    }
}

fn line_column(source: &str, position: usize) -> (usize, usize) {
    let position = position.min(source.len());
    let prefix = &source[..position];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit_once('\n').map_or_else(
        || prefix.chars().count() + 1,
        |(_, tail)| tail.chars().count() + 1,
    );
    (line, column)
}

fn parse_condition(expression: &str) -> Result<Condition> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Err(Error::EmptyExpression);
    }
    let (negated, path) = match expression.strip_prefix('!') {
        Some(path) => (true, path.trim()),
        None => (false, expression),
    };
    validate_path(path)?;
    Ok(Condition {
        path: path.to_owned(),
        negated,
    })
}

fn validate_path(path: &str) -> Result<()> {
    if path.is_empty() {
        return Err(Error::EmptyExpression);
    }
    if path.split('.').all(is_identifier) {
        Ok(())
    } else {
        Err(Error::InvalidExpression(path.to_owned()))
    }
}

fn validate_identifier(value: &str) -> Result<()> {
    if is_identifier(value) {
        Ok(())
    } else {
        Err(Error::InvalidExpression(value.to_owned()))
    }
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}
