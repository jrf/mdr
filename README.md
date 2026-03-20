# mdr

[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)](https://www.rust-lang.org/)

A terminal markdown reader with live file watching.

## Features

- Renders markdown with styled headings, bold, italic, code blocks, blockquotes, lists, task lists, and horizontal rules
- Live reload — file changes are reflected instantly
- Vim-style scrolling (`j`/`k`, `g`/`G`, `Ctrl-f`/`Ctrl-b`)
- File picker overlay — press `f`, type to filter, enter to open
- Search — `/` to search, `n`/`N` to cycle matches
- Multiple color themes, cycled with `t`
- External editor — press `e` to open in `$EDITOR`
- Word wrapping with list continuation indent

## Install

```bash
# Directly from GitHub
cargo install --git https://github.com/jrf/mdr

# With just
just install

# Or manually
cargo build --release
cp target/release/mdr ~/.local/bin/
```

## Usage

```
mdr <file.md>
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `Ctrl-f` / `PageDown` | Page down |
| `Ctrl-b` / `PageUp` | Page up |
| `g` / `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `Enter` | Fold/unfold section |
| `x` / `Space` | Toggle task checkbox |
| `Tab` / `Shift-Tab` | Next / previous unchecked task |
| `F` | Toggle task filter view |
| `/` | Search |
| `n` / `N` | Next/previous match |
| `f` | File picker (opens in new tab) |
| `t` | Cycle theme |
| `e` | Open in `$EDITOR` |
| `H` / `L` | Previous / next tab |
| `W` | Close current tab |
| `?` | Help |
| `q` / `Ctrl-c` | Quit |

### Themes

synthwave, monochrome, ocean, sunset, matrix, tokyo night moon

### Configuration

Settings are stored in `~/.config/mdr/config.toml`:

```toml
theme = "tokyo night moon"
scrollbar = true
```

## Requirements

- Rust (stable)
- A terminal with 256-color support
