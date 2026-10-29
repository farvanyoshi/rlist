use chrono::{Days, NaiveDate};
use std::path::{Path, PathBuf};
use std::{env, fs, io};

const DATE_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl Priority {
    pub fn letter(self) -> char {
        match self {
            Priority::High => 'H',
            Priority::Medium => 'M',
            Priority::Low => 'L',
        }
    }

    pub fn parse(s: &str) -> Option<Priority> {
        match s.to_lowercase().as_str() {
            "h" | "high" => Some(Priority::High),
            "m" | "med" | "medium" => Some(Priority::Medium),
            "l" | "low" => Some(Priority::Low),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub done: bool,
    pub priority: Option<Priority>,
    pub due: Option<NaiveDate>,
    pub text: String,
}

/// Turns raw user input into a single-line item text; `None` if it is blank.
pub fn clean_text(raw: &str) -> Option<String> {
    let text = raw.replace(['\n', '\r', '\t'], " ");
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

impl Item {
    pub fn new(text: &str) -> Option<Item> {
        Some(Item {
            done: false,
            priority: None,
            due: None,
            text: clean_text(text)?,
        })
    }

    pub fn is_overdue(&self, today: NaiveDate) -> bool {
        !self.done && self.due.is_some_and(|due| due < today)
    }

    /// File format: `[ ] (H) due:2026-11-01 text`; priority and due are optional.
    pub fn to_line(&self) -> String {
        let mut line = String::from(if self.done { "[x] " } else { "[ ] " });
        if let Some(p) = self.priority {
            line.push_str(&format!("({}) ", p.letter()));
        }
        if let Some(due) = self.due {
            line.push_str(&format!("due:{} ", due.format(DATE_FORMAT)));
        }
        line.push_str(&self.text);
        line
    }

    /// Lines without a `[ ]`/`[x]` prefix are plain items from older lists.
    pub fn from_line(line: &str) -> Option<Item> {
        let line = line.trim_end();
        let (done, mut rest) = if let Some(rest) = line.strip_prefix("[ ] ") {
            (false, rest)
        } else if let Some(rest) = line.strip_prefix("[x] ") {
            (true, rest)
        } else {
            return Item::new(line);
        };

        let priority = match rest.get(..4) {
            Some("(H) ") => Some(Priority::High),
            Some("(M) ") => Some(Priority::Medium),
            Some("(L) ") => Some(Priority::Low),
            _ => None,
        };
        if priority.is_some() {
            rest = &rest[4..];
        }

        let mut due = None;
        if let Some((date, remainder)) = rest.strip_prefix("due:").and_then(|r| r.split_once(' '))
            && let Ok(date) = NaiveDate::parse_from_str(date, DATE_FORMAT)
        {
            due = Some(date);
            rest = remainder;
        }

        Some(Item {
            done,
            priority,
            due,
            text: clean_text(rest)?,
        })
    }
}

/// `RLIST_FILE`, else `~/.rlist/todo-list.txt`, else `./todo-list.txt` when there is no home.
pub fn list_path() -> PathBuf {
    if let Some(path) = env::var_os("RLIST_FILE").filter(|p| !p.is_empty()) {
        return PathBuf::from(path);
    }
    match env::var_os("HOME").filter(|h| !h.is_empty()) {
        Some(home) => PathBuf::from(home).join(".rlist").join("todo-list.txt"),
        None => PathBuf::from("todo-list.txt"),
    }
}

/// A missing file is an empty list.
pub fn load(path: &Path) -> io::Result<Vec<Item>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents.lines().filter_map(Item::from_line).collect()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

pub fn save(path: &Path, items: &[Item]) -> io::Result<()> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    let mut contents = String::new();
    for item in items {
        contents.push_str(&item.to_line());
        contents.push('\n');
    }
    // Write beside the list and rename so a failed write cannot truncate it
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)
}

/// Accepts `YYYY-MM-DD`, `today`, `tomorrow`, and `+Nd` / `+Nw`.
pub fn parse_date(s: &str, today: NaiveDate) -> Result<NaiveDate, String> {
    let lower = s.to_lowercase();
    let invalid = || format!("invalid date '{s}' (use YYYY-MM-DD, today, tomorrow, +3d or +2w)");
    match lower.as_str() {
        "today" => Ok(today),
        "tomorrow" => today.checked_add_days(Days::new(1)).ok_or_else(invalid),
        other => match other.strip_prefix('+') {
            Some(offset) => {
                let (number, per_unit) = match offset.chars().last() {
                    Some('d') => (&offset[..offset.len() - 1], 1),
                    Some('w') => (&offset[..offset.len() - 1], 7),
                    _ => return Err(invalid()),
                };
                let days: u64 = number.parse().map_err(|_| invalid())?;
                days.checked_mul(per_unit)
                    .and_then(|d| today.checked_add_days(Days::new(d)))
                    .ok_or_else(invalid)
            }
            None => NaiveDate::parse_from_str(other, DATE_FORMAT).map_err(|_| invalid()),
        },
    }
}

/// Resolves item numbers (1-based) or exact item text to indices into `items`.
/// Returns the distinct indices found and an error message for each argument that matched nothing.
pub fn resolve_targets(items: &[Item], args: &[String]) -> (Vec<usize>, Vec<String>) {
    let mut found: Vec<usize> = Vec::new();
    let mut errors = Vec::new();

    for arg in args {
        let by_number = if !arg.is_empty() && arg.chars().all(|c| c.is_ascii_digit()) {
            match arg.parse::<usize>() {
                Ok(n) if (1..=items.len()).contains(&n) => Some(n - 1),
                _ => None,
            }
        } else {
            None
        };

        if let Some(index) = by_number {
            if !found.contains(&index) {
                found.push(index);
            }
            continue;
        }

        // Text match takes the first copy not already chosen, so `a a` hits two duplicates
        let by_text = (0..items.len()).find(|i| items[*i].text == *arg && !found.contains(i));
        match by_text {
            Some(index) => found.push(index),
            None if arg.chars().all(|c| c.is_ascii_digit()) && !arg.is_empty() => {
                errors.push(format!("no item number {arg}"));
            }
            None => errors.push(format!("item not found: {arg}")),
        }
    }
    (found, errors)
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ListOpts {
    pub sort: bool,
    pub pending: bool,
    pub done: bool,
    pub overdue: bool,
    pub priority: Option<Priority>,
    pub query: Option<String>,
}

/// Indices of the items to show, filtered and (optionally) sorted. Indices stay original positions.
pub fn select(items: &[Item], opts: &ListOpts, today: NaiveDate) -> Vec<usize> {
    let query = opts.query.as_ref().map(|q| q.to_lowercase());
    let mut selected: Vec<usize> = (0..items.len())
        .filter(|&i| {
            let item = &items[i];
            (!opts.pending || !item.done)
                && (!opts.done || item.done)
                && (!opts.overdue || item.is_overdue(today))
                && opts.priority.is_none_or(|p| item.priority == Some(p))
                && query
                    .as_ref()
                    .is_none_or(|q| item.text.to_lowercase().contains(q))
        })
        .collect();

    if opts.sort {
        // Pending first, then priority High..Low (none last), then earliest due (none last)
        selected.sort_by_key(|&i| {
            let item = &items[i];
            (
                item.done,
                item.priority.map_or(3, |p| p as u8),
                item.due.is_none(),
                item.due,
                i,
            )
        });
    }
    selected
}

pub fn format_item(number: usize, item: &Item, today: NaiveDate, color: bool) -> String {
    let mut line = format!("{number}. [{}] ", if item.done { 'x' } else { ' ' });
    if let Some(p) = item.priority {
        line.push_str(&format!("({}) ", p.letter()));
    }
    line.push_str(&item.text);
    let overdue = item.is_overdue(today);
    if let Some(due) = item.due {
        let marker = if overdue { ", OVERDUE" } else { "" };
        line.push_str(&format!("  (due {}{marker})", due.format(DATE_FORMAT)));
    }

    if !color {
        return line;
    }
    if item.done {
        format!("\x1b[2m{line}\x1b[0m")
    } else if overdue {
        format!("\x1b[31m{line}\x1b[0m")
    } else {
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn item(text: &str) -> Item {
        Item::new(text).unwrap()
    }

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn line_round_trip_keeps_every_field() {
        let full = Item {
            done: true,
            priority: Some(Priority::Medium),
            due: Some(date(2026, 11, 1)),
            text: "buy milk".into(),
        };
        assert_eq!(full.to_line(), "[x] (M) due:2026-11-01 buy milk");
        assert_eq!(Item::from_line(&full.to_line()), Some(full));

        let plain = item("plain");
        assert_eq!(plain.to_line(), "[ ] plain");
        assert_eq!(Item::from_line(&plain.to_line()), Some(plain));
    }

    #[test]
    fn legacy_and_blank_lines() {
        assert_eq!(Item::from_line("old style item"), Some(item("old style item")));
        assert_eq!(Item::from_line(""), None);
        assert_eq!(Item::from_line("   "), None);
        assert_eq!(Item::from_line("[ ] "), Some(item("[ ]")));
    }

    #[test]
    fn bad_due_date_stays_in_text() {
        let parsed = Item::from_line("[ ] due:2026-13-45 something").unwrap();
        assert_eq!(parsed.due, None);
        assert_eq!(parsed.text, "due:2026-13-45 something");
    }

    #[test]
    fn blank_text_is_rejected_and_newlines_flattened() {
        assert!(Item::new("   ").is_none());
        assert_eq!(clean_text("a\nb\tc").unwrap(), "a b c");
    }

    #[test]
    fn dates() {
        let today = date(2026, 10, 29);
        assert_eq!(parse_date("today", today), Ok(today));
        assert_eq!(parse_date("Tomorrow", today), Ok(date(2026, 10, 30)));
        assert_eq!(parse_date("+3d", today), Ok(date(2026, 11, 1)));
        assert_eq!(parse_date("+2w", today), Ok(date(2026, 11, 12)));
        assert_eq!(parse_date("2027-01-05", today), Ok(date(2027, 1, 5)));
        for bad in ["2026-02-30", "soon", "+d", "+3x", "+-1d", ""] {
            assert!(parse_date(bad, today).is_err(), "{bad} should fail");
        }
    }

    #[test]
    fn targets_by_number_and_text() {
        let items = vec![item("a"), item("b"), item("a"), item("2024")];
        let (found, errors) = resolve_targets(&items, &strings(&["2", "a", "a", "a"]));
        assert_eq!(found, vec![1, 0, 2]);
        assert_eq!(errors, vec!["item not found: a"]);

        let (found, errors) = resolve_targets(&items, &strings(&["9", "2024", "zzz", "2"]));
        assert_eq!(found, vec![3, 1]);
        assert_eq!(errors, vec!["no item number 9", "item not found: zzz"]);
    }

    #[test]
    fn numbers_resolve_against_original_positions() {
        let items = vec![item("a"), item("b"), item("c")];
        let (found, _) = resolve_targets(&items, &strings(&["1", "3"]));
        assert_eq!(found, vec![0, 2]);
    }

    #[test]
    fn filters_and_sort() {
        let today = date(2026, 10, 29);
        let mut items = vec![
            item("plain"),
            Item { priority: Some(Priority::Low), ..item("low") },
            Item { priority: Some(Priority::High), due: Some(date(2026, 12, 1)), ..item("high late") },
            Item { priority: Some(Priority::High), due: Some(date(2026, 10, 1)), ..item("high overdue") },
            Item { done: true, priority: Some(Priority::High), ..item("finished") },
        ];

        let sorted = select(&items, &ListOpts { sort: true, ..Default::default() }, today);
        assert_eq!(sorted, vec![3, 2, 1, 0, 4]);

        let overdue = select(&items, &ListOpts { overdue: true, ..Default::default() }, today);
        assert_eq!(overdue, vec![3]);

        let pending = select(&items, &ListOpts { pending: true, ..Default::default() }, today);
        assert_eq!(pending, vec![0, 1, 2, 3]);

        let done = select(&items, &ListOpts { done: true, ..Default::default() }, today);
        assert_eq!(done, vec![4]);

        let high = select(&items, &ListOpts { priority: Some(Priority::High), ..Default::default() }, today);
        assert_eq!(high, vec![2, 3, 4]);

        let search = select(&items, &ListOpts { query: Some("HIGH".into()), ..Default::default() }, today);
        assert_eq!(search, vec![2, 3]);

        items[3].done = true;
        let overdue = select(&items, &ListOpts { overdue: true, ..Default::default() }, today);
        assert!(overdue.is_empty(), "done items are never overdue");
    }

    #[test]
    fn formatting() {
        let today = date(2026, 10, 29);
        let late = Item {
            priority: Some(Priority::High),
            due: Some(date(2026, 10, 1)),
            ..item("pay rent")
        };
        assert_eq!(
            format_item(2, &late, today, false),
            "2. [ ] (H) pay rent  (due 2026-10-01, OVERDUE)"
        );
        let done = Item { done: true, ..late };
        assert_eq!(format_item(2, &done, today, false), "2. [x] (H) pay rent  (due 2026-10-01)");
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = env::temp_dir().join(format!("rlist-test-{}", std::process::id()));
        let path = dir.join("nested").join("list.txt");
        assert!(load(&path).unwrap().is_empty());

        let items = vec![item("one"), Item { done: true, ..item("two") }];
        save(&path, &items).unwrap();
        assert_eq!(load(&path).unwrap(), items);
        fs::remove_dir_all(dir).unwrap();
    }
}
