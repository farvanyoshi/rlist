use crate::list::{ListOpts, Priority, parse_date};
use chrono::NaiveDate;

#[derive(Debug, PartialEq, Eq)]
pub struct Command {
    /// Set by `-l` (or a bare `rlist`): print the list before the action runs
    pub list_first: Option<ListOpts>,
    pub action: Action,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Add { texts: Vec<String>, priority: Option<Priority>, due: Option<NaiveDate> },
    Done(Vec<String>),
    Undone(Vec<String>),
    Remove(Vec<String>),
    Clear { done_only: bool },
    Edit(EditArgs),
    Search(ListOpts),
    Help,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EditArgs {
    pub target: String,
    pub text: Option<String>,
    /// `Some(None)` means "clear it"
    pub priority: Option<Option<Priority>>,
    pub due: Option<Option<NaiveDate>>,
    pub interactive: bool,
}

pub fn usage() -> &'static str {
    "Usage: rlist [-l [list options]] [action]

With no arguments, rlist lists everything. -l lists first, then runs the action.

Actions:
  -a TEXT...      Add items. Options: -p high|med|low, --due DATE
  -d ITEM...      Mark items done.
  -u ITEM...      Mark items not done.
  -r ITEM...      Remove items.
  -e ITEM [TEXT]  Edit an item. Options: -p high|med|low|none, --due DATE|none
                  With no TEXT or options, or with -i, edit its text interactively.
  -c [--done]     Clear the whole list, or only the completed items.
  -s WORD...      Search item text (case-insensitive).
  -h              Show this message.

ITEM is an item number (as shown by -l) or the item's exact text.
DATE is YYYY-MM-DD, today, tomorrow, +3d or +2w.

List options (for -l and -s):
  --sort          Pending first, then by priority, then by due date.
  --pending       Only items not done.
  --done          Only completed items.
  --overdue       Only pending items past their due date.
  -p, --priority  Only items with this priority (high|med|low)."
}

/// `args` excludes the program name.
pub fn parse(args: &[String], today: NaiveDate) -> Result<Command, String> {
    if args.is_empty() {
        return Ok(Command { list_first: Some(ListOpts::default()), action: Action::None });
    }

    let (list_first, rest) = if args[0] == "-l" {
        let (opts, rest) = leading_list_opts(&args[1..])?;
        (Some(opts), rest)
    } else {
        (None, args)
    };

    let action = match rest.split_first() {
        None => Action::None,
        Some((flag, rest)) => parse_action(flag, rest, today)?,
    };
    Ok(Command { list_first, action })
}

fn parse_action(flag: &str, args: &[String], today: NaiveDate) -> Result<Action, String> {
    match flag {
        "-h" | "--help" => Ok(Action::Help),
        "-a" => {
            let scan = scan(args, today)?;
            if scan.interactive {
                return Err("-i only applies to -e".into());
            }
            if scan.positional.is_empty() {
                return Err("nothing to add".into());
            }
            Ok(Action::Add {
                texts: scan.positional,
                priority: scan.priority.flatten(),
                due: scan.due.flatten(),
            })
        }
        "-d" => targets(args, "mark done").map(Action::Done),
        "-u" => targets(args, "mark not done").map(Action::Undone),
        "-r" => targets(args, "remove").map(Action::Remove),
        "-c" => match args {
            [] => Ok(Action::Clear { done_only: false }),
            [opt] if opt == "--done" => Ok(Action::Clear { done_only: true }),
            _ => Err("-c only accepts --done".into()),
        },
        "-e" => {
            let mut scan = scan(args, today)?;
            if scan.positional.is_empty() {
                return Err("-e needs an item number or text to edit".into());
            }
            let target = scan.positional.remove(0);
            let text = (!scan.positional.is_empty()).then(|| scan.positional.join(" "));
            // Nothing to change on the command line means the user wants the editor
            let interactive =
                scan.interactive || (text.is_none() && scan.priority.is_none() && scan.due.is_none());
            Ok(Action::Edit(EditArgs {
                target,
                text,
                priority: scan.priority,
                due: scan.due,
                interactive,
            }))
        }
        "-s" => {
            let mut opts = ListOpts::default();
            let mut words = Vec::new();
            let mut i = 0;
            while i < args.len() {
                match list_opt(&args[i..], &mut opts)? {
                    Some(used) => i += used,
                    None => {
                        words.push(args[i].clone());
                        i += 1;
                    }
                }
            }
            if words.is_empty() {
                return Err("nothing to search for".into());
            }
            opts.query = Some(words.join(" "));
            Ok(Action::Search(opts))
        }
        _ => Err("unknown flag".into()),
    }
}

fn targets(args: &[String], verb: &str) -> Result<Vec<String>, String> {
    if args.is_empty() {
        return Err(format!("nothing to {verb}"));
    }
    Ok(args.to_vec())
}

/// Consumes list options at the front of `args`, returning the rest.
fn leading_list_opts(args: &[String]) -> Result<(ListOpts, &[String]), String> {
    let mut opts = ListOpts::default();
    let mut i = 0;
    while i < args.len() {
        match list_opt(&args[i..], &mut opts)? {
            Some(used) => i += used,
            None => break,
        }
    }
    Ok((opts, &args[i..]))
}

/// If `args` starts with a list option, applies it and returns how many arguments it used.
fn list_opt(args: &[String], opts: &mut ListOpts) -> Result<Option<usize>, String> {
    match args[0].as_str() {
        "--sort" => opts.sort = true,
        "--pending" => opts.pending = true,
        "--done" => opts.done = true,
        "--overdue" => opts.overdue = true,
        "-p" | "--priority" => {
            let value = args.get(1).ok_or("--priority needs a value")?;
            let priority = Priority::parse(value)
                .ok_or_else(|| format!("invalid priority '{value}' (use high, med or low)"))?;
            opts.priority = Some(priority);
            return Ok(Some(2));
        }
        _ => return Ok(None),
    }
    if opts.pending && opts.done {
        return Err("--pending and --done cannot be combined".into());
    }
    Ok(Some(1))
}

#[derive(Default)]
struct Scan {
    positional: Vec<String>,
    priority: Option<Option<Priority>>,
    due: Option<Option<NaiveDate>>,
    interactive: bool,
}

/// Splits add/edit arguments into options and plain words. `--` makes the rest plain words.
fn scan(args: &[String], today: NaiveDate) -> Result<Scan, String> {
    let mut scan = Scan::default();
    let mut literal = false;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        i += 1;
        if literal {
            scan.positional.push(arg.to_string());
            continue;
        }
        match arg {
            "--" => literal = true,
            "-i" | "--interactive" => scan.interactive = true,
            "-p" | "--priority" => {
                let value = args.get(i).ok_or("--priority needs a value")?;
                i += 1;
                scan.priority = Some(if value.eq_ignore_ascii_case("none") {
                    None
                } else {
                    Some(Priority::parse(value).ok_or_else(|| {
                        format!("invalid priority '{value}' (use high, med, low or none)")
                    })?)
                });
            }
            "--due" => {
                let value = args.get(i).ok_or("--due needs a value")?;
                i += 1;
                scan.due = Some(if value.eq_ignore_ascii_case("none") {
                    None
                } else {
                    Some(parse_date(value, today)?)
                });
            }
            _ => scan.positional.push(arg.to_string()),
        }
    }
    Ok(scan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 29).unwrap()
    }

    fn parse_str(line: &str) -> Result<Command, String> {
        let args: Vec<String> = line.split_whitespace().map(String::from).collect();
        parse(&args, today())
    }

    #[test]
    fn no_arguments_lists() {
        let cmd = parse_str("").unwrap();
        assert_eq!(cmd.list_first, Some(ListOpts::default()));
        assert_eq!(cmd.action, Action::None);
    }

    #[test]
    fn list_then_action() {
        let cmd = parse_str("-l --sort -a buy milk -p high --due tomorrow").unwrap();
        assert_eq!(cmd.list_first, Some(ListOpts { sort: true, ..Default::default() }));
        assert_eq!(
            cmd.action,
            Action::Add {
                texts: vec!["buy".into(), "milk".into()],
                priority: Some(Priority::High),
                due: NaiveDate::from_ymd_opt(2026, 10, 30),
            }
        );
    }

    #[test]
    fn edit_variants() {
        let Action::Edit(edit) = parse_str("-e 2").unwrap().action else { panic!() };
        assert!(edit.interactive && edit.text.is_none());

        let Action::Edit(edit) = parse_str("-e 2 new words --due none").unwrap().action else { panic!() };
        assert!(!edit.interactive);
        assert_eq!(edit.text.as_deref(), Some("new words"));
        assert_eq!(edit.due, Some(None));

        let Action::Edit(edit) = parse_str("-e 2 -p low").unwrap().action else { panic!() };
        assert!(!edit.interactive);
        assert_eq!(edit.priority, Some(Some(Priority::Low)));
    }

    #[test]
    fn double_dash_allows_dash_words() {
        let Action::Add { texts, .. } = parse_str("-a -- -p stays").unwrap().action else { panic!() };
        assert_eq!(texts, vec!["-p", "stays"]);
    }

    #[test]
    fn search_with_options() {
        let Action::Search(opts) = parse_str("-s milk --pending").unwrap().action else { panic!() };
        assert_eq!(opts.query.as_deref(), Some("milk"));
        assert!(opts.pending);
    }

    #[test]
    fn errors() {
        for bad in [
            "-x", "-a", "-r", "-d", "-u", "-e", "-s", "-c --all", "-a x -p urgent", "-a x --due",
            "-a x --due 2026-02-30", "-l --pending --done", "-a x -i",
        ] {
            assert!(parse_str(bad).is_err(), "'{bad}' should fail");
        }
    }
}
