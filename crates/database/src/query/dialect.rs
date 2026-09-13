use crate::{DatabaseError, Driver, ErrorKind, Result};

pub(super) fn identifier(driver: Driver, value: &str) -> Result<String> {
    if value == "*" {
        return Ok(value.into());
    }
    let quote = match driver {
        Driver::MySql => '`',
        Driver::Postgres | Driver::Sqlite => '"',
    };
    let mut output = String::new();
    for (index, part) in value.split('.').enumerate() {
        if part.is_empty() || (part != "*" && !valid_part(part)) {
            return Err(DatabaseError::new(
                ErrorKind::Query,
                format!("invalid SQL identifier: {value}"),
            ));
        }
        if index > 0 {
            output.push('.');
        }
        if part == "*" {
            output.push('*');
        } else {
            output.push(quote);
            output.push_str(part);
            output.push(quote);
        }
    }
    Ok(output)
}

pub(super) fn placeholder(driver: Driver, position: usize) -> String {
    match driver {
        Driver::Postgres => format!("${position}"),
        Driver::MySql | Driver::Sqlite => "?".into(),
    }
}

fn valid_part(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

pub(super) fn operator(value: &str) -> Result<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "=" => Ok("="),
        "!=" => Ok("!="),
        "<>" => Ok("<>"),
        ">" => Ok(">"),
        ">=" => Ok(">="),
        "<" => Ok("<"),
        "<=" => Ok("<="),
        "like" => Ok("LIKE"),
        "not like" => Ok("NOT LIKE"),
        _ => Err(DatabaseError::new(
            ErrorKind::Query,
            format!("unsupported SQL operator: {value}"),
        )),
    }
}
