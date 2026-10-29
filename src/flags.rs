use std::fs::{self, OpenOptions};
use std::io::Write;

const LIST_FILE: &str = "todo-list.txt";

pub fn print_usage() {
    println!("Usage: rlist [-l] [-a | -c | -r] args...");
    println!("  -l:   List all items in list before operation.");
    println!("  -a:   Add args to list.");
    println!("  -c:   Clear all items from list.");
    println!("  -r:   Remove args from list.");
    println!("  -h:   Show this message.");
}

pub fn match_flag(flag: &str, args: &[String]) {
    match flag {
        "-a" => add_items(args),
        "-c" => clear_items(),
        "-r" => remove_items(args),
        "-h" => print_usage(),
        _ => eprintln!("Error: unknown flag. See usage with -h."),
    }
}

fn add_items(items: &[String]) {
    // If passed with no further arguments
    if items.is_empty() {
        eprintln!("Error: nothing to add.");
        return;
    }

    // Open file in append mode, create if doesn't exist
    let mut list = match OpenOptions::new().create(true).append(true).open(LIST_FILE) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error: could not open or create file: {}", e);
            return;
        }
    };

    // Write each element to file, but continue writing if one fails
    for item in items {
        match list.write_all(format!("{item}\n").as_bytes()) {
            Ok(()) => println!("Item added: {}", item),
            Err(e) => eprintln!("Error: could not append to file: {}", e),
        }
    }
}

fn clear_items() {
    match fs::metadata(LIST_FILE) {
        Ok(_) => match fs::write(LIST_FILE, "") {
            Ok(()) => println!("List cleared."),
            Err(e) => eprintln!("Error: could not clear file: {}", e),
        },
        Err(_) => println!("No to-do list found."),
    }
}

fn remove_items(items: &[String]) {
    if items.is_empty() {
        eprintln!("Error: nothing to remove.");
        return;
    }

    let contents = match fs::read_to_string(LIST_FILE) {
        Ok(contents) => contents,
        Err(_) => {
            println!("No to-do list found.");
            return;
        }
    };

    // Keep every line not matching a requested item; track which were found
    let mut found = vec![false; items.len()];
    let mut kept: Vec<&str> = Vec::new();
    for line in contents.lines() {
        match items.iter().position(|item| item == line) {
            Some(i) => found[i] = true,
            None => kept.push(line),
        }
    }

    let mut output = kept.join("\n");
    if !kept.is_empty() {
        output.push('\n');
    }
    if let Err(e) = fs::write(LIST_FILE, output) {
        eprintln!("Error: could not write file: {}", e);
        return;
    }

    for (item, was_found) in items.iter().zip(found) {
        if was_found {
            println!("Item removed: {}", item);
        } else {
            eprintln!("Item not found: {}", item);
        }
    }
}
