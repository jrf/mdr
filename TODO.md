# TODO

## Now

- [x] Auto-refresh file picker when files are added/removed in the watched directory
- [ ] Task-aware navigation (`]t` / `[t` to jump between unchecked tasks)
- [x] Task summary in status bar (e.g., `3/7 tasks`)

## Next

- [x] Filter view — `f` to collapse document to only unchecked task lines (with heading context)
- [x] Show task counts next to `.md` files in browser mode (e.g., `project-plan.md  [3/7]`)
- [x] Sort/group browser by files with open tasks
- [ ] File watcher notification — flash `[updated]` in status bar when file reloads

## Later

- [ ] Append mode — `a` to quick-add a `- [x] task` without opening an editor
- [ ] Syntax highlighting in fenced code blocks (syntect or tree-sitter)
- [ ] Mouse scroll support (crossterm already emits mouse events)
- [ ] Accept stdin (`cat file.md | mdr`)
- [x] Persist selected theme to config file (`~/.config/mdr/config.toml`)
- [ ] Follow markdown links — open URLs in browser, jump to local `.md` files
- [ ] Browser preview pane (split layout showing selected file)
- [ ] Fuzzy search/filter in browser mode
- [ ] Scrollbar widget
- [ ] Footnote rendering
- [ ] Tidy command — keybind to move completed `[x]` tasks to a "Done" section

## Done

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
