# mdr

[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)](https://www.rust-lang.org/)

A terminal markdown reader with live file watching.

## Features

- Renders markdown with styled headings, bold, italic, code blocks, blockquotes, lists, task lists, and horizontal rules
- Live reload — file changes are reflected instantly
- Vim-style scrolling (`j`/`k`, `g`/`G`, `Ctrl-d`/`Ctrl-u`)
- Multiple color themes, cycled with `t`
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
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `Ctrl-d` | Page down |
| `Ctrl-u` | Page up |
| `g` | Go to top |
| `G` | Go to bottom |
| `t` | Cycle theme |
| `q` / `Ctrl-c` | Quit |

### Themes

synthwave, monochrome, ocean, sunset, matrix, tokyo night moon

## Requirements

- Rust (stable)
- A terminal with 256-color support
