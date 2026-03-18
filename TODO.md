# TODO

## Now

- [ ] Use `unicode-width` for line length calculations (dependency exists but `current_line_len` uses `.len()`, breaks on wide/CJK chars)
- [ ] Use actual visible height for `Ctrl-d`/`Ctrl-u` page size in reader mode (currently hardcoded to 20)
- [ ] Clamp `scroll_bottom` properly instead of setting `usize::MAX`
- [ ] Render tables (pulldown-cmark parses them but they're silently dropped)
- [ ] Render strikethrough (option is enabled but not handled)

## Next

- [ ] Cache parsed markdown — only re-parse when content changes, not on every scroll/redraw
- [ ] Search (`/`) with highlighted matches and `n`/`N` navigation
- [ ] Syntax highlighting in fenced code blocks (syntect or tree-sitter)
- [ ] Mouse scroll support (crossterm already emits mouse events)
- [ ] Accept stdin (`cat file.md | meld`)
- [ ] Persist selected theme to config file

## Later

- [ ] Follow markdown links — open URLs in browser, jump to local `.md` files
- [ ] Browser preview pane (split layout showing selected file)
- [ ] Fuzzy search/filter in browser mode
- [ ] Scrollbar widget
- [ ] Footnote rendering
