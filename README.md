# rlist

A small command-line to-do list. Items live in a plain text file, and can have a priority, a due date, and a done state.

## Install

```bash
cargo build --release          # binary at target/release/rlist
cargo install --path .         # or install it onto your PATH
```

## Quick start

```bash
rlist -a "buy milk" "call mom"                      # add items
rlist -a "pay rent" -p high --due 2026-11-01        # add with priority and due date
rlist -a "dentist" --due +2w                        # relative dates work too
rlist                                               # list everything
rlist -l --sort                                     # most urgent first
rlist -d 1                                          # mark item 1 done
rlist -e 2                                          # edit item 2 in place
rlist -r "buy milk"                                 # remove by exact text
rlist -l -a "water plants"                          # list, then add
```

Example output:

```
1. [x] buy milk
2. [ ] call mom
3. [ ] (H) pay rent  (due 2026-10-01, OVERDUE)
4. [ ] dentist  (due 2026-11-12)
4 items (1 done)
```

Items are always shown with their position in the list. That number is what `-d`, `-u`, `-r` and `-e` take, and it stays the same even when the output is filtered or sorted.

## Commands

Run `rlist` with no arguments to list everything.

| Command | What it does |
|---|---|
| `-a TEXT...` | Add one item per argument. Options: `-p PRIORITY`, `--due DATE` apply to every item added. |
| `-d ITEM...` | Mark items done. |
| `-u ITEM...` | Mark items not done. |
| `-r ITEM...` | Remove items. |
| `-e ITEM [TEXT]` | Edit an item. Give new `TEXT` and/or `-p`, `--due`; or edit interactively (below). |
| `-c` | Clear the whole list. |
| `-c --done` | Clear only completed items. |
| `-s WORD...` | Search item text (case-insensitive). Accepts the list options below. |
| `-l [options] [command]` | List first, then run the command, if given. |
| `-h` | Show usage. |

`ITEM` is an item number as shown in the list, or the item's exact text. A number is tried first; if no such item exists it is matched as text. Several items can be given at once (`rlist -r 1 3` removes the first and third items). If the same text appears twice, text matching takes one copy per argument.

If an argument can't be handled (an unknown number, text that isn't found), the rest are still applied, the problems are printed to stderr, and the exit code is 1.

### List options (`-l`, `-s`)

| Option | Shows |
|---|---|
| `--sort` | Pending items first, then by priority (high to low), then by earliest due date; items without a priority or due date go last. |
| `--pending` | Only items that are not done. |
| `--done` | Only completed items. |
| `--overdue` | Only pending items whose due date has passed. |
| `-p`, `--priority P` | Only items with that priority. |

Options can be combined, for example `rlist -l --pending --sort`. Overdue items are labelled `OVERDUE`, and shown in red when output is a terminal (set `NO_COLOR` to turn colours off).

### Priorities and dates

- Priorities are `high`, `med` and `low` (or `h`, `m`, `l`), shown as `(H)`, `(M)`, `(L)`.
- Dates are `YYYY-MM-DD`, `today`, `tomorrow`, `+3d` (days) or `+2w` (weeks).
- With `-e`, use `-p none` or `--due none` to remove a priority or due date.

### Editing

```bash
rlist -e 2                       # interactive: the current text is pre-filled to edit
rlist -e 2 -i                    # same, spelled out
rlist -e 2 "call mom tonight"    # replace the text directly
rlist -e 2 -p low --due +3d      # change only the priority and due date
```

The interactive editor shows the item's text on the prompt line. Use the arrow keys, Home/End and the usual line-editing shortcuts, then press Enter to save. Ctrl-C or Ctrl-D cancels and leaves the item unchanged. A blank result is rejected. Interactive editing needs a terminal; in scripts pass the new text instead.

An argument that starts with `-` is treated as an option. To add text such as `-5 degrees`, put `--` before it: `rlist -a -- "-5 degrees"`.

## Where the list is stored

By default the list is `~/.rlist/todo-list.txt`, so it is the same list from any directory. Set `RLIST_FILE` to use a different file:

```bash
RLIST_FILE=./groceries.txt rlist -a eggs
```

If you used an earlier version that kept `todo-list.txt` in the current directory, move it: `mkdir -p ~/.rlist && mv todo-list.txt ~/.rlist/`. Old plain lists load fine; items without a state are treated as not done.

The file is plain text with one item per line, so you can read or edit it by hand:

```
[ ] buy milk
[x] call mom
[ ] (H) due:2026-11-01 pay rent
```

Each line is `[ ]` or `[x]`, then an optional `(H)`/`(M)`/`(L)`, then an optional `due:YYYY-MM-DD`, then the text.

## Exit codes

`0` on success. `1` when a command is invalid or any part of it failed; the reason is printed to stderr.

## Development

```bash
cargo test
```

Unit tests cover the file format, date parsing, sorting and filtering, and argument parsing. The tests in `tests/cli.rs` run the real binary against a temporary list file.
