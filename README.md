# Berserk

A modern Rust framework for building secure, maintainable, and production-ready applications.

## Installation

Add Berserk to your project's `Cargo.toml`:

```toml
[dependencies]
berserk = "0.1.0"
```

Then build your application:

```bash
cargo build
```

## Requirements

- Rust 1.88 or later
- Cargo
- A supported database when using database features

## Optional Features

Enable only the integrations your application needs:

```toml
[dependencies]
berserk = { version = "0.1.0", features = ["postgres"] }
```

Available features depend on the adapters provided by Berserk.

## Documentation

Additional documentation is available in the [`docs`](docs) directory.

## Security

Please report security vulnerabilities privately according to [`SECURITY.md`](SECURITY.md).

## Contributing

Contributions are welcome. Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before submitting a pull request.

## License

Berserk is distributed under the license specified in [`LICENSE`](LICENSE).
