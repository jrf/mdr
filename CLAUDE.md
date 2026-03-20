# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build              # debug build
cargo build --release    # release build
cargo run -- <file.md>   # run directly, opens in reader mode
cargo run                # no args: opens file picker
just install             # release build + codesign + copy to ~/.local/bin/
just run <file.md>       # shorthand for cargo run
```

No tests or linter are configured yet.

## Architecture

mdr is a terminal markdown reader built with Rust, ratatui, and crossterm. It renders markdown in the terminal with syntax highlighting and live-reloads on file changes.

**Five app modes** (`AppMode` in `state.rs`): `Reader` is the primary mode. `FilePicker`, `Search`, `ThemePicker`, and `Help` are overlays drawn on top of the reader. `ThemePicker` stores the original theme index so it can revert on cancel. All overlay modes return to `Reader` on dismiss.

**Single-threaded event loop** (`main.rs`): Uses `crossterm::event::poll` for input and an `AtomicBool` flag (set by the notify file watcher) for file changes. Only redraws when state actually changes. Global keybindings (`q`, `Ctrl-c`) are handled first, then mode-specific bindings are dispatched.

**Rendering pipeline**: `ui::draw` always renders the reader first, then overlays the active modal (file picker, theme picker, or help) on top. The reader calls `markdown::parse_markdown` which converts the full markdown source into a `Vec<StyledLine>` using pulldown-cmark. The UI then slices this by scroll offset and renders via ratatui. Parsed output is cached per-tab and only re-parsed when content, theme, or terminal width changes.

**Tab system** (`state.rs`): Per-file state is stored in `Tab` structs (content, scroll, cursor, search, cache, folds). `AppState` holds a `Vec<Tab>` and `active_tab` index. `tab()`/`tab_mut()` accessors provide the active tab. The file picker opens files in new tabs (or switches to an existing tab if the file is already open). `H`/`L` switch tabs, `W` closes the current tab (minimum 1 tab). The file watcher watches all open tab file paths and updates whichever tab's file changed.

**Key modules**:
- `main.rs` — event loop, file watcher setup (watches all tab file paths), terminal init/cleanup, external editor launch.
- `state.rs` — `Tab` holds per-file state: content, cursor/scroll, search, parsed line cache, folds. `AppState` holds mode, tabs, active tab index, theme, browser, and scrollbar config. Navigation and editing methods live on `Tab`.
- `browser.rs` — `BrowserState` manages directory listing (dirs first, then `.md` files, hidden files excluded), selection, scroll, and type-to-filter for the file picker overlay.
- `markdown.rs` — pulldown-cmark event loop (using `into_offset_iter()`) producing styled, word-wrapped lines. Handles headings, code blocks, blockquotes, lists (ordered/unordered), task lists, inline formatting, and horizontal rules. Each `StyledLine` carries an optional `source_line` for task items, enabling checkbox toggling.
- `ui.rs` — ratatui rendering: reader with status bar, plus centered popup overlays for file picker, theme picker, and help. Search highlighting is applied post-parse in `highlight_search`.
- `theme.rs` — six color themes using 256-color indexed palette (`ALL_THEMES` array). Default is "tokyo night moon" (index 5). Themes are selected at runtime via the theme picker (`t`).
- `config.rs` — persists user settings to `~/.config/mdr/config.toml`. TOML format with `theme` and `scrollbar` keys. `Config` struct loaded at startup, saved on theme confirm.

**Key dependencies**: ratatui 0.30, crossterm 0.28, pulldown-cmark 0.12, notify 7, unicode-width 0.2. Version matters — ratatui and crossterm APIs change significantly between major versions.

**Search system** (`state.rs`): Case-insensitive substring search across raw content lines. `open_search()` enters Search mode, `update_search()` scans on each keystroke, `search_next()`/`search_prev()` cycle matches. Highlighting is applied in `ui.rs` via `highlight_search`. Search state is per-tab.

**File picker** (`f` key): Opens a centered overlay listing `.md` files and directories from the current file's parent dir. Supports type-to-filter, directory navigation, and opens selected files in a new tab (or switches to existing tab if already open).

**External editor** (`main.rs`): In reader mode, `e` suspends the TUI, launches `$EDITOR` (or `vi`) on the current file, then restores the TUI and reloads content.

## Workflow Rules

- **TODO.md must be kept meticulously updated.** When marking a task `[x]`, always move it from its current section to Done in the same edit. Never leave checked items in Now/Next/Later. When discovering new work, add it to the appropriate section.
- **README.md must be kept meticulously updated.** When adding or changing features, keybindings, configuration options, or setup instructions, update the README to match.
