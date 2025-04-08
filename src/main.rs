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
    // Try to read file and place in string list, else stop the program
    let list: Vec<String> = fs::read_to_string("todo-list.txt")
        .unwrap()
        .lines()
        .map(String::from)
        .collect();

    // format and print list to console
    println!("Number of items in list: {}", list.len());
    for item in list {
        println!("{}", item);
    }

    if let Some(flag) = args.get(2) {
        flags::match_flag(flag, &args[ARG_OFFSET..]);
    }
}