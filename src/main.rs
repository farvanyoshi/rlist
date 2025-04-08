use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;

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
        other => match_flag(other, &args[ARG_OFFSET..]) // first arg is name, second is flag, so the rest will be commands
    }
}

fn print_usage() {
    println!("Usage: rlist [-l] [-a | -c | -r] args...");
    println!("  -l:   List all items in list before operation.");
    println!("  -a:   Add args to list.");
    println!("  -c:   Clear all items from list.");
    println!("  -r:   Remove args from list.");
    return;
}

fn match_flag(flag: &str, args: &[String]) {
    match flag {
        "-a" => add_items(args),
        "-c" => todo!(),
        "-r" => todo!(),
        "-h" => print_usage(),
        _ => {
            eprintln!("Error: unknown flag. See usage with -h.");
            return;
        }
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
        match_flag(flag, &args[ARG_OFFSET..]);
    }
}

fn add_items(items: &[String]) {
    // If passed with no further arguments
    if items.is_empty() {
        eprintln!("Error: nothing to add.");
        return;
    }

    // Open file in append mode, create if doesn't exist
    let mut list = match OpenOptions::new()
        .create(true)
        .append(true)
        .open("todo-list.txt") {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Error: could not open or create file: {}", e);
                return;
            }
    };

    // Write each element to file, but continue writing if one fails
    for item in items {
        if let Err(e) = list.write(format!("{item}\n").as_bytes()) {
            eprintln!("Error: could not append to file: {}", e);
        }
        println!("Item added: {}", item);
    }
}