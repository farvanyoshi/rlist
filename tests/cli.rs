use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        let dir = std::env::temp_dir().join(format!("rlist-cli-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Sandbox { dir }
    }

    fn file(&self) -> PathBuf {
        self.dir.join("list.txt")
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rlist"))
            .args(args)
            .env("RLIST_FILE", self.file())
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    }

    fn ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(out.status.success(), "{args:?} failed: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8(out.stdout).unwrap()
    }

    fn fails(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(!out.status.success(), "{args:?} should fail");
        String::from_utf8(out.stderr).unwrap()
    }

    fn contents(&self) -> String {
        fs::read_to_string(self.file()).unwrap_or_default()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn add_list_done_remove() {
    let s = Sandbox::new("basic");
    assert_eq!(s.ok(&[]), "No items in list.\n");
    s.ok(&["-a", "milk", "eggs", "bread"]);
    assert_eq!(s.ok(&["-l"]), "1. [ ] milk\n2. [ ] eggs\n3. [ ] bread\n3 items (0 done)\n");

    s.ok(&["-d", "2", "bread"]);
    assert_eq!(s.contents(), "[ ] milk\n[x] eggs\n[x] bread\n");
    assert!(s.ok(&["-l", "--pending"]).contains("Showing 1 of 3 items (2 done)"));

    s.ok(&["-u", "eggs"]);
    s.ok(&["-r", "1", "3"]);
    assert_eq!(s.contents(), "[ ] eggs\n");
}

#[test]
fn chained_list_does_not_add_the_flag_itself() {
    let s = Sandbox::new("chain");
    let out = s.ok(&["-l", "-a", "one"]);
    assert!(out.starts_with("No items in list."));
    assert_eq!(s.contents(), "[ ] one\n");
}

#[test]
fn priorities_due_dates_and_sorting() {
    let s = Sandbox::new("prio");
    s.ok(&["-a", "plain"]);
    s.ok(&["-a", "rent", "-p", "high", "--due", "2020-01-01"]);
    s.ok(&["-a", "gym", "-p", "low", "--due", "2099-01-01"]);
    assert_eq!(s.contents(), "[ ] plain\n[ ] (H) due:2020-01-01 rent\n[ ] (L) due:2099-01-01 gym\n");

    let sorted = s.ok(&["-l", "--sort"]);
    let order: Vec<&str> = sorted.lines().take(3).map(|l| l.split(' ').next().unwrap()).collect();
    assert_eq!(order, ["2.", "3.", "1."]);

    let overdue = s.ok(&["-l", "--overdue"]);
    assert!(overdue.contains("2. [ ] (H) rent  (due 2020-01-01, OVERDUE)"));
    assert!(overdue.contains("Showing 1 of 3 items"));
}

#[test]
fn scripted_edit() {
    let s = Sandbox::new("edit");
    s.ok(&["-a", "old", "-p", "high", "--due", "2099-01-01"]);
    s.ok(&["-e", "1", "new", "text", "--due", "none"]);
    assert_eq!(s.contents(), "[ ] (H) new text\n");
    s.ok(&["-e", "1", "-p", "none"]);
    assert_eq!(s.contents(), "[ ] new text\n");
}

#[test]
fn interactive_edit_needs_a_terminal() {
    let s = Sandbox::new("interactive");
    s.ok(&["-a", "item"]);
    assert!(s.fails(&["-e", "1"]).contains("needs a terminal"));
    assert!(s.fails(&["-e", "1", "-i"]).contains("needs a terminal"));
    assert_eq!(s.contents(), "[ ] item\n");
}

#[test]
fn search_and_clear() {
    let s = Sandbox::new("search");
    s.ok(&["-a", "Buy milk", "call mom", "buy eggs"]);
    s.ok(&["-d", "1"]);
    let found = s.ok(&["-s", "BUY"]);
    assert!(found.contains("1. [x] Buy milk") && found.contains("3. [ ] buy eggs"));
    assert!(!found.contains("call mom"));
    assert!(s.ok(&["-s", "nothing"]).contains("No matching items."));

    assert!(s.ok(&["-c", "--done"]).contains("Cleared 1 completed item."));
    assert_eq!(s.contents(), "[ ] call mom\n[ ] buy eggs\n");
    assert!(s.ok(&["-c"]).contains("List cleared."));
    assert_eq!(s.contents(), "");
}

#[test]
fn old_plain_text_lists_still_load() {
    let s = Sandbox::new("legacy");
    fs::write(s.file(), "first\nsecond\n").unwrap();
    assert_eq!(s.ok(&["-l"]), "1. [ ] first\n2. [ ] second\n2 items (0 done)\n");
}

#[test]
fn errors_exit_nonzero_and_write_nothing() {
    let s = Sandbox::new("errors");
    assert!(s.fails(&["-x"]).contains("unknown flag"));
    assert!(s.fails(&["-a"]).contains("nothing to add"));
    assert!(s.fails(&["-a", "x", "--due", "someday"]).contains("invalid date"));
    assert!(s.fails(&["-a", "ok", "   "]).contains("blank"));
    assert!(!s.file().exists(), "failed adds must not create the file");

    s.ok(&["-a", "keep"]);
    let err = s.fails(&["-r", "keep", "9", "ghost"]);
    assert!(err.contains("no item number 9") && err.contains("item not found: ghost"));
    assert_eq!(s.contents(), "", "valid targets are still removed");
}

#[test]
fn help_prints_usage() {
    let s = Sandbox::new("help");
    assert!(s.ok(&["-h"]).starts_with("Usage: rlist"));
}
