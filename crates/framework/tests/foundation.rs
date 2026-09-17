use berserk::{App, Error, ServerConfig, Validate};
use std::{error::Error as _, time::Duration};

#[test]
fn defaults_are_valid_and_configuration_is_retained() {
    ServerConfig::default().validate().unwrap();
    let app = App::with_config(ServerConfig {
        workers: 2,
        max_body_bytes: 0,
        ..ServerConfig::default()
    })
    .unwrap();
    assert_eq!(app.config().workers, 2);
    assert_eq!(app.config().max_body_bytes, 0);
    assert_eq!(App::new().config().workers, ServerConfig::default().workers);
}

#[test]
fn invalid_settings_preserve_the_error_source() {
    let error = App::with_config(ServerConfig {
        workers: 0,
        ..ServerConfig::default()
    })
    .unwrap_err();
    assert!(error.source().is_some());
    match error {
        Error::Configuration(error) => assert_eq!(error.field(), "workers"),
        _ => panic!("unexpected error category"),
    }
}

#[test]
fn rejects_zero_limits_and_timeouts() {
    let cases = [
        (
            ServerConfig {
                queue_capacity: 0,
                ..ServerConfig::default()
            },
            "queue_capacity",
        ),
        (
            ServerConfig {
                max_header_bytes: 0,
                ..ServerConfig::default()
            },
            "max_header_bytes",
        ),
        (
            ServerConfig {
                max_headers: 0,
                ..ServerConfig::default()
            },
            "max_headers",
        ),
        (
            ServerConfig {
                read_timeout: Duration::ZERO,
                ..ServerConfig::default()
            },
            "read_timeout",
        ),
        (
            ServerConfig {
                write_timeout: Duration::ZERO,
                ..ServerConfig::default()
            },
            "write_timeout",
        ),
        (
            ServerConfig {
                request_deadline: Duration::ZERO,
                ..ServerConfig::default()
            },
            "request_deadline",
        ),
    ];
    for (config, field) in cases {
        assert_eq!(config.validate().unwrap_err().field(), field);
    }
}

#[test]
fn rejects_overflowing_combined_request_limit() {
    let config = ServerConfig {
        max_body_bytes: usize::MAX,
        ..ServerConfig::default()
    };
    assert_eq!(config.validate().unwrap_err().field(), "max_body_bytes");
}
