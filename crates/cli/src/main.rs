#![forbid(unsafe_code)]

use berserk_cli::{execute, Command, Generator, UnsupportedMigrations};

fn main() {
    let result = std::env::current_dir()
        .map_err(berserk_cli::CliError::from_io)
        .and_then(Generator::at)
        .and_then(|generator| {
            Command::parse(std::env::args().skip(1))
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
