use std::env;
use std::process::ExitCode;

mod cli;
mod flags;
mod list;

// A to-do list stored in a plain text file; see README.md for usage

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect(); // skip the program name
    let today = chrono::Local::now().date_naive();

    let command = match cli::parse(&args, today) {
        Ok(command) => command,
        Err(e) => {
            eprintln!("Error: {e}. See usage with -h.");
            return ExitCode::FAILURE;
        }
    };

    match flags::run(command, &list::list_path(), today) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
