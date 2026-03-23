# mdr

[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)](https://www.rust-lang.org/)

A terminal markdown reader with live file watching.

## Features

- Renders markdown with styled headings, bold, italic, code blocks, blockquotes, lists, task lists, horizontal rules, and `[[wikilinks]]`
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
| `Enter` | Fold/unfold section, follow link |
| `[` / `]` | Fold all / unfold all |
| `x` / `Space` | Toggle task checkbox |
| `Ctrl-n` / `Ctrl-p` | Next / previous unchecked task |
| `u` | Toggle unchecked task filter |
| `l` | Filter by label |
| `o` | Outline / table of contents |
| `b` | Toggle bookmark |
| `B` | Bookmark list |
| `'` / `"` | Next / previous bookmark |
| `/` | Search |
| `n` / `N` | Next/previous match |
| `f` | File picker (opens in new tab) |
| `t` | Cycle theme |
| `e` | Open in `$EDITOR` |
| `Tab` / `Shift-Tab` | Next / previous tab |
| `?` | Help |
| `q` | Close tab / quit |
| `Ctrl-c` | Quit |

### Themes

synthwave, monochrome, ocean, sunset, matrix, tokyo night moon

### Configuration

Settings are stored in `~/.config/mdr/config.toml`:

```toml
theme = "tokyo night moon"
scrollbar = true
```

Themes live in `~/.config/mdr/themes/` as individual `.toml` files. The theme name is derived from the filename (hyphens become spaces, e.g. `tokyo-night-moon.toml` → "tokyo night moon").

Example theme file (`~/.config/mdr/themes/tokyo-night-moon.toml`):

```toml
[colors]
bg = "#222436"
bg_highlight = "#2f334d"
fg = "#c8d3f5"
fg_bright = "#d5dff5"
comment = "#636da6"
fg_gutter = "#3b4261"
red = "#ff757f"
yellow = "#ffc777"
green = "#c3e88d"
blue = "#82aaff"
magenta = "#c099ff"

[ui]
border = "fg_gutter"
accent = "magenta"
text = "fg"
text_bright = "fg_bright"
text_dim = "comment"
text_muted = "fg_gutter"
heading = "blue"
error = "red"
cursor_bg = "bg_highlight"

[labels]
bugs = "red"
features = "green"
improvements = "magenta"
refactor = "yellow"
docs = "blue"
chore = "comment"
```

- Each theme file has a `[colors]` palette (hex `#rrggbb`), a `[ui]` section mapping roles to palette names, and a `[labels]` section for category colors
- All fields are optional — missing fields fall back to a hardcoded default
- Add new themes by dropping a `.toml` file in the themes directory
- Requires a truecolor-capable terminal

## Requirements

- Rust (stable)
- A terminal with truecolor (24-bit) support
