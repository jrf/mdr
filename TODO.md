# TODO

## Now

- [ ] Filter tasks by tag — keybind to show only lines matching a tag #feature
- [ ] Task summary in status bar (e.g., `3/7 tasks`) #feature

## Next

- [ ] Syntax highlighting in fenced code blocks (syntect or tree-sitter) #feature
- [ ] Accept stdin (`cat file.md | mdr`) #feature
- [ ] Follow markdown links — open URLs in browser, jump to local `.md` files #feature
- [ ] Table of contents overlay — list headings, jump to any section #feature
- [ ] Browser preview pane (split layout showing selected file) #improvement
- [ ] Bookmarks — mark positions in a file, jump back to them #feature
- [ ] Inline image rendering via Sixel (with fallback to `[image: alt text]` placeholder) #feature
- [ ] Footnote rendering #feature
- [ ] Tidy command — keybind to move completed `[x]` tasks to a "Done" section #feature
- [ ] Indented outline mode — optional indent of content under headings #improvement

## Later

## Done

- [x] Inline `#tag` rendering with color-coded category labels #feature
- [x] Fold all / unfold all with `[` / `]` #feature
- [x] Independent section folding — each heading folds its own content #improvement
- [x] `q` closes tab, quits on last tab #improvement
- [x] Configurable themes — per-file TOML with named color palettes #feature
- [x] Category label colors in theme config #feature
- [x] Themes loaded from `~/.config/mdr/themes/*.toml` #feature
- [x] Replaced hand-rolled config parser with serde + toml #refactor
- [x] Truecolor (24-bit RGB) theme support #improvement
- [x] Multiple file tabs — `Tab`/`Shift-Tab` to switch, `q` to close #feature
- [x] Search with `n`/`N` match navigation #feature
- [x] Toggle checkboxes in-place with `x` or `Space` #feature
- [x] Cursor-based navigation with line highlighting #feature
- [x] Filter view — `F` to show only unchecked tasks #feature
- [x] Fuzzy search in file picker #feature
- [x] Outline-aware folding with `Enter` #feature
- [x] Unicode-width line length calculations #improvement
- [x] Correct page size for `Ctrl-f`/`Ctrl-b` #improvement
- [x] Scroll clamping fix #bug
- [x] Parsed markdown caching #improvement
- [x] Mouse scroll support #improvement
- [x] Scrollbar widget #feature
- [x] Auto-refresh file picker on directory changes #improvement
- [x] Task navigation with `Ctrl-n`/`Ctrl-p` #feature
- [x] Persist selected theme to config #feature
- [x] File watcher with `[updated]` indicator #feature
- [x] Table rendering #feature
- [x] Strikethrough rendering #feature
