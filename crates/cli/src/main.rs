#![forbid(unsafe_code)]

use berserk_cli::{execute, Command, Generator, UnsupportedMigrations};

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() == 1 && matches!(arguments[0].as_str(), "--version" | "-V") {
        println!("berserk {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let result = std::env::current_dir()
        .map_err(berserk_cli::CliError::from_io)
        .and_then(Generator::at)
        .and_then(|generator| {
            Command::parse(arguments)
                .and_then(|command| execute(command, &generator, &UnsupportedMigrations))
        });
    match result {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(2);
        }
    }
}
