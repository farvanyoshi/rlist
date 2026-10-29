use std::env;
use std::fs;
pub mod flags;

// The goal is to write a basic to-do list with a file to store data

// offset for getting arguments from command line
const ARG_OFFSET: usize = 2;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Error: No arguments specified. Use -h to see usage.");
        return;
    }

    match (&args[1]).as_str() {
        "-l" => {
            list_all(args);
            return;
        },
        other => flags::match_flag(other, &args[ARG_OFFSET..]) // first arg is name, second is flag, so the rest will be commands
    }
}

fn list_all(args: Vec<String>) {
    match fs::read_to_string("todo-list.txt") {
        Ok(contents) => {
            let items: Vec<&str> = contents.lines().collect();
            println!("Number of items in list: {}", items.len());
            for item in items {
                println!("{}", item);
            }
            if let Some(flag) = args.get(2) {
                flags::match_flag(flag, &args[ARG_OFFSET + 1..]); // skip the flag itself
            }
        }
        Err(_) => println!("No to-do list found.")
    };
}