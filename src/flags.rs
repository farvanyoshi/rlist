use crate::cli::{Action, Command, EditArgs, usage};
use crate::list::{self, Item, ListOpts, clean_text, format_item, resolve_targets};
use chrono::NaiveDate;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::io::{self, IsTerminal};
use std::path::Path;

/// Runs a parsed command against the list file. `Err` carries the full text to print on stderr
/// and means a non-zero exit, even when part of the command succeeded.
pub fn run(cmd: Command, path: &Path, today: NaiveDate) -> Result<(), String> {
    if let Action::Help = cmd.action {
        println!("{}", usage());
        return Ok(());
    }

    let color = io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let mut items = list::load(path)
        .map_err(|e| format!("Error: could not read {}: {e}", path.display()))?;

    if let Some(opts) = &cmd.list_first {
        print_list(&items, opts, today, color);
    }

    let (changed, problems) = match cmd.action {
        Action::None | Action::Help => (false, Vec::new()),
        Action::Search(opts) => {
            print_list(&items, &opts, today, color);
            (false, Vec::new())
        }
        Action::Add { texts, priority, due } => add_items(&mut items, texts, priority, due)?,
        Action::Done(targets) => set_done(&mut items, &targets, true),
        Action::Undone(targets) => set_done(&mut items, &targets, false),
        Action::Remove(targets) => remove_items(&mut items, &targets),
        Action::Clear { done_only } => clear_items(&mut items, done_only),
        Action::Edit(args) => edit_item(&mut items, args, today, color)?,
    };

    if changed {
        list::save(path, &items)
            .map_err(|e| format!("Error: could not write {}: {e}", path.display()))?;
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.iter().map(|p| format!("Error: {p}")).collect::<Vec<_>>().join("\n"))
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn print_list(items: &[Item], opts: &ListOpts, today: NaiveDate, color: bool) {
    if items.is_empty() {
        println!("No items in list.");
        return;
    }
    let selected = list::select(items, opts, today);
    if selected.is_empty() {
        println!("No matching items.");
        return;
    }
    for &i in &selected {
        println!("{}", format_item(i + 1, &items[i], today, color));
    }
    let done = items.iter().filter(|item| item.done).count();
    if selected.len() == items.len() {
        println!("{} item{} ({done} done)", items.len(), plural(items.len()));
    } else {
        println!("Showing {} of {} items ({done} done)", selected.len(), items.len());
    }
}

type Outcome = (bool, Vec<String>);

fn add_items(
    items: &mut Vec<Item>,
    texts: Vec<String>,
    priority: Option<list::Priority>,
    due: Option<NaiveDate>,
) -> Result<Outcome, String> {
    // Validate everything first so a blank argument writes nothing
    let mut new_items = Vec::new();
    for text in &texts {
        let text = clean_text(text).ok_or("Error: cannot add a blank item.")?;
        new_items.push(Item { done: false, priority, due, text });
    }
    for item in new_items {
        println!("Item added: {}", item.text);
        items.push(item);
    }
    Ok((true, Vec::new()))
}

fn set_done(items: &mut [Item], targets: &[String], done: bool) -> Outcome {
    let (found, problems) = resolve_targets(items, targets);
    for &i in &found {
        items[i].done = done;
        let label = if done { "Marked done" } else { "Marked not done" };
        println!("{label}: {}", items[i].text);
    }
    (!found.is_empty(), problems)
}

fn remove_items(items: &mut Vec<Item>, targets: &[String]) -> Outcome {
    let (mut found, problems) = resolve_targets(items, targets);
    // Remove from the back so earlier indices stay valid
    found.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &found {
        let item = items.remove(i);
        println!("Item removed: {}", item.text);
    }
    (!found.is_empty(), problems)
}

fn clear_items(items: &mut Vec<Item>, done_only: bool) -> Outcome {
    if done_only {
        let before = items.len();
        items.retain(|item| !item.done);
        let cleared = before - items.len();
        println!("Cleared {cleared} completed item{}.", plural(cleared));
        (cleared > 0, Vec::new())
    } else {
        let changed = !items.is_empty();
        items.clear();
        println!("List cleared.");
        (changed, Vec::new())
    }
}

fn edit_item(
    items: &mut [Item],
    args: EditArgs,
    today: NaiveDate,
    color: bool,
) -> Result<Outcome, String> {
    let (found, problems) = resolve_targets(items, std::slice::from_ref(&args.target));
    let Some(&index) = found.first() else {
        return Ok((false, problems));
    };

    let mut text = match &args.text {
        Some(text) => clean_text(text).ok_or("Error: item text cannot be blank.")?,
        None => items[index].text.clone(),
    };
    if args.interactive {
        match prompt_edit(&text)? {
            Some(edited) => text = clean_text(&edited).ok_or("Error: item text cannot be blank.")?,
            None => {
                println!("Edit cancelled.");
                return Ok((false, Vec::new()));
            }
        }
    }

    let item = &mut items[index];
    item.text = text;
    if let Some(priority) = args.priority {
        item.priority = priority;
    }
    if let Some(due) = args.due {
        item.due = due;
    }
    println!("Item updated: {}", format_item(index + 1, item, today, color));
    Ok((true, Vec::new()))
}

/// Shows `current` on the prompt line for editing. `None` means the user cancelled.
fn prompt_edit(current: &str) -> Result<Option<String>, String> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("Error: interactive edit needs a terminal; pass the new text instead.".into());
    }
    let mut editor = DefaultEditor::new().map_err(|e| format!("Error: could not start editor: {e}"))?;
    match editor.readline_with_initial("> ", (current, "")) {
        Ok(line) => Ok(Some(line)),
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => Ok(None),
        Err(e) => Err(format!("Error: could not read input: {e}")),
    }
}
