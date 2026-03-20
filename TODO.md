# TODO

## Now

- [ ] Task summary in status bar (e.g., `3/7 tasks`)

## Next

## Later

- [ ] Syntax highlighting in fenced code blocks (syntect or tree-sitter)
- [ ] Accept stdin (`cat file.md | mdr`)
- [ ] Follow markdown links — open URLs in browser, jump to local `.md` files
- [ ] Browser preview pane (split layout showing selected file)
- [ ] Table of contents overlay — list headings, jump to any section
- [ ] Inline image rendering via Sixel (with fallback to `[image: alt text]` placeholder)
- [ ] Footnote rendering
- [ ] Bookmarks — mark positions in a file, jump back to them
- [ ] Tidy command — keybind to move completed `[x]` tasks to a "Done" section

## Done

- [x] Multiple file tabs — open several files, switch with `H`/`L`, close with `W`
- [x] Use `unicode-width` for line length calculations
- [x] Use actual visible height for `Ctrl-d`/`Ctrl-u` page size in reader mode
- [x] Clamp `scroll_bottom` properly instead of setting `usize::MAX`
- [x] Render tables
- [x] Render strikethrough
- [x] Search (`/`) with highlighted matches and `n`/`N` navigation
- [x] Cache parsed markdown — only re-parse when content changes, not on every scroll/redraw
- [x] Toggle checkboxes in-place (`x` or `Space` on a task line) — flip `[ ]` / `[x]` and write back to file
- [x] Cursor-based navigation with line highlighting
- [x] Persist selected theme to config file
- [x] File watcher notification — show `[updated]` in status bar when file reloads (clears on next keypress)
- [x] Filter view — `F` to collapse document to only unchecked task lines (with heading context)
- [x] Auto-refresh file picker when files are added/removed in the watched directory
- [x] Task-aware navigation (`Tab` / `Shift-Tab` to jump between unchecked tasks)
- [x] Mouse scroll support (scrolls viewport, clamps cursor)
- [x] Scrollbar widget (configurable via `scrollbar` in config.toml)
- [x] Fuzzy search/filter in file picker
- [x] Outline-aware folding — `Enter` to collapse/expand sections by heading
