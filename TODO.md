# TODO

## Now

## Next

- [ ] Task summary in status bar (e.g., `3/7 tasks`) #feature
- [ ] Browser preview pane (split layout showing selected file) #improvement
- [ ] Inline image rendering via Sixel (with fallback to `[image: alt text]` placeholder) #feature
- [ ] Footnote rendering #feature
- [ ] Tidy command — keybind to move completed `[x]` tasks to a "Done" section #feature
- [ ] Indented outline mode — optional indent of content under headings #improvement
- [ ] Heading jump with `{`/`}` — jump to previous/next heading like vim paragraph motion #feature

## Later

- [ ] LaTeX math Unicode approximation — render `$...$` and `$$...$$` with Greek letters, super/subscripts, and common symbols #feature

## Scrapped

- H3 headings as category labels — replaced by inline `#tags`, more flexible
- `F` as unified filter picker — two-step UX felt clunky, split into `u`/`l` instead

## Done

- [x] Table of contents overlay — `o` to list headings, jump to any section #feature
- [x] Follow markdown links — Enter on `.md` links opens in tab, URLs open browser #feature
- [x] Bookmarks — `m` to toggle, `'`/`"` to cycle #feature
- [x] Inline `#tag` rendering with color-coded category labels #feature
- [x] Fold all / unfold all with `[` / `]` #feature
- [x] Independent section folding — each heading folds its own content #improvement
- [x] `q` closes tab, quits on last tab #improvement
- [x] Syntax highlighting in fenced code blocks via syntect #feature
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
