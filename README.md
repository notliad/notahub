# notahub

**notahub** is a keyboard-driven TUI for managing projects, tasks, and ideas —
all stored as plain markdown files.

Everything is a file. No database, no lock-in.

## Install

### From source (requires Rust)

```sh
cargo install --git https://github.com/notliad/notahub
```

Or clone and build:

```sh
git clone https://github.com/notliad/notahub
cd notahub
cargo build --release
cp target/release/notahub ~/.local/bin/
```

### Homebrew (macOS / Linux)

```sh
brew install YOUR_TAP/notahub
```

### Arch Linux (AUR)

```sh
yay -S notahub
```

### Nix

```sh
nix profile github:notliad/notahub
```

### Windows (Scoop)

```powershell
scoop bucket add notahub https://github.com/notliad/scoop-notahub
scoop install notahub
```

### Binary releases

Download the latest binary for your platform from
[releases](https://github.com/notliad/notahub/releases) and place it in
your `$PATH`.

## Usage

```
notahub
```

Data directory (auto-created):

| OS      | Path                                |
|---------|-------------------------------------|
| Linux   | `$NOTAHUB_HOME` or `~/.local/share/notahub/` |
| macOS   | `$NOTAHUB_HOME` or `~/Library/Application Support/notahub/` |
| Windows | `$NOTAHUB_HOME` or `%APPDATA%/notahub/` |

Override with `NOTAHUB_HOME`.

### Keybindings

| Key          | Action                        |
|--------------|-------------------------------|
| `1` `2` `3`  | Switch screen                 |
| `h` `l`      | Switch column / pane          |
| `j` `k`      | Move selection                |
| `Enter`      | Open selected item            |
| `Esc`        | Back / close modal            |
| `/`          | Search                        |
| `p`          | New project                   |
| `i`          | New idea                      |
| `t`          | New task                      |
| `n`          | New note                      |
| `Space`      | Toggle task completed         |
| `x`          | Delete mode                   |
| `?`          | Help                          |
| `Q`          | Quit                          |

Item shortcuts (`a b c …`) navigate or select items. When more than 17
items exist, two-letter combos (`aa ab …`) are used. Letters reserved for
actions (`h i j k l n p t x`) are never assigned as item shortcuts.

### Storage

Projects and ideas are plain markdown files with YAML frontmatter:

```
---
id: <uuid>
title: My Project
created: 2026-01-01T00:00:00Z
---

- [ ] Task title <!-- id:<uuid> -->
  Notes go here
```

- `projects/` — one file per project
- `ideas/` — one file per idea

Edit them with any editor. notahub reads and writes them directly.

## Development

```sh
cargo test
cargo run
```
