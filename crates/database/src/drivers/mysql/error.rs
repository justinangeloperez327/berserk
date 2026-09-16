use crate::{DatabaseError, ErrorKind};

pub(super) fn map_error(error: mysql::Error) -> DatabaseError {
    let (kind, code) = match &error {
        mysql::Error::IoError(_)
        | mysql::Error::CodecError(_)
        | mysql::Error::DriverError(_)
        | mysql::Error::TlsError(_) => (ErrorKind::Connection, None),

        mysql::Error::UrlError(_) => (ErrorKind::Configuration, None),

        mysql::Error::FromValueError(_) | mysql::Error::FromRowError(_) => {
            (ErrorKind::Decode, None)
        }

        mysql::Error::MySqlError(server) => {
            let code = server.code.to_string();
            let kind = match server.code {
                1062 | 1216 | 1217 | 1451 | 1452 => ErrorKind::Constraint,
                1205 | 1213 => ErrorKind::Serialization,
                _ => ErrorKind::Query,
            };

            (kind, Some(code))
        }
    };

    let mapped = DatabaseError::new(kind, error.to_string());

    match code {
        Some(code) => mapped.with_code(code),
        None => mapped,
    }
}
