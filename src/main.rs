mod browser;
mod config;
mod markdown;
mod state;
mod theme;
mod ui;

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind, EnableMouseCapture, DisableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use state::{AppMode, AppState};

/// Set up a file watcher that watches the parent directories of all given paths.
fn setup_watcher(paths: &[PathBuf], flag: Arc<AtomicBool>) -> Option<RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            if event.kind.is_modify() {
                flag.store(true, Ordering::Relaxed);
            }
        }
    })
    .ok()?;

    let mut watched: HashSet<PathBuf> = HashSet::new();
    for path in paths {
        let dir = path.parent().unwrap_or(path).to_path_buf();
        if watched.insert(dir.clone()) {
            let _ = watcher.watch(&dir, RecursiveMode::NonRecursive);
        }
    }

    Some(watcher)
}

fn setup_dir_watcher(dir: &PathBuf, flag: Arc<AtomicBool>) -> Option<RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            if event.kind.is_create() || event.kind.is_remove() {
                flag.store(true, Ordering::Relaxed);
            }
        }
    })
    .ok()?;

    watcher.watch(dir, RecursiveMode::NonRecursive).ok()?;

    Some(watcher)
}

/// Rebuild the file watcher to watch all open tab file paths.
fn rebuild_watcher(state: &AppState, flag: Arc<AtomicBool>) -> Option<RecommendedWatcher> {
    let paths = state.tab_file_paths();
    if paths.is_empty() {
        return None;
    }
    setup_watcher(&paths, flag)
}

fn main() -> io::Result<()> {
    let file_arg = env::args().nth(1);

    let cfg = config::load_config();
    let theme_configs = config::load_theme_configs();
    let themes = theme::resolve_themes(&theme_configs);
    let initial_theme = cfg.theme.as_deref()
        .and_then(|name| theme::find_theme(&themes, name))
        .map(|(idx, _)| idx)
        .unwrap_or(0);

    let mut state = if let Some(ref arg) = file_arg {
        let file_path = PathBuf::from(arg).canonicalize().map_err(|e| {
            eprintln!("error: {}: {}", arg, e);
            e
        })?;
        let content = fs::read_to_string(&file_path)?;
        AppState::new_reader(file_path, content, initial_theme, themes, cfg.scrollbar)
    } else {
        let dir = env::current_dir()?;
        AppState::new_picker(dir, initial_theme, themes, cfg.scrollbar)
    };

    // File change flag (set by watcher, cleared by main loop)
    let file_dirty = Arc::new(AtomicBool::new(false));
    let dir_dirty = Arc::new(AtomicBool::new(false));

    // Set up watcher for all open tab files
    let mut _watcher: Option<RecommendedWatcher> = rebuild_watcher(&state, file_dirty.clone());

    // Set up directory watcher for the file picker
    let mut _dir_watcher: Option<RecommendedWatcher> =
        Some(setup_dir_watcher(&state.browser.current_dir, dir_dirty.clone())).flatten();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let mut needs_redraw = true;
    loop {
        if needs_redraw {
            terminal.draw(|f| ui::draw(f, &mut state))?;
            needs_redraw = false;
        }

        // Check for file changes — check all tabs
        if file_dirty.swap(false, Ordering::Relaxed) {
            for tab in &mut state.tabs {
                if let Some(ref path) = tab.file_path {
                    if let Ok(new_content) = fs::read_to_string(path) {
                        if new_content != tab.content {
                            tab.content = new_content;
                            tab.file_updated = true;
                            needs_redraw = true;
                        }
                    }
                }
            }
        }

        // Check if background recursive file list is ready
        if state.browser.poll_recursive() {
            needs_redraw = true;
        }

        // Check for directory changes (refresh file picker entries)
        if dir_dirty.swap(false, Ordering::Relaxed) {
            state.browser.refresh();
            needs_redraw = true;
        }

        // Poll for terminal events
        if event::poll(Duration::from_millis(50))? {
            if let Ok(ev) = event::read() {
                match ev {
                    Event::Key(key) => {
                        needs_redraw = true;
                        state.tab_mut().file_updated = false;
                        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                        match key.code {
                            KeyCode::Char('c') if ctrl => break,
                            _ => match state.mode {
                                AppMode::Reader => match key.code {
                                    KeyCode::Char('q') => {
                                        if state.tabs.len() <= 1 {
                                            break;
                                        }
                                        state.close_tab();
                                    }
                                    KeyCode::Esc if !state.tab().search_query.is_empty() => {
                                        let tab = state.tab_mut();
                                        tab.search_query.clear();
                                        tab.search_matches.clear();
                                        tab.search_current = 0;
                                    }
                                    KeyCode::Char('f') if !ctrl => {
                                        state.browser.filter.clear();
                                        state.browser.rebuild_filter();
                                        state.browser.preload_recursive();
                                        state.mode = AppMode::FilePicker;
                                    }
                                    KeyCode::Char('u') => {
                                        let tab = state.tab_mut();
                                        tab.filter_tasks = !tab.filter_tasks;
                                        tab.cursor = 0;
                                        tab.scroll = 0;
                                    }
                                    KeyCode::Char('l') => state.open_label_picker(),
                                    KeyCode::Char('o') => state.open_toc(),
                                    KeyCode::Char('b') if !ctrl => state.tab_mut().toggle_bookmark(),
                                    KeyCode::Char('B') => state.open_bookmark_list(),
                                    KeyCode::Char('\'') => state.tab_mut().next_bookmark(),
                                    KeyCode::Char('"') => state.tab_mut().prev_bookmark(),
                                    KeyCode::Char('t') => state.open_theme_picker(),
                                    KeyCode::Char('?') => state.open_help(),
                                    KeyCode::Char('/') => {
                                        state.tab_mut().open_search();
                                        state.mode = AppMode::Search;
                                    }
                                    KeyCode::Char('n') if ctrl => state.tab_mut().next_task(),
                                    KeyCode::Char('p') if ctrl => state.tab_mut().prev_task(),
                                    KeyCode::Char('n') => state.tab_mut().search_next(),
                                    KeyCode::Char('N') => state.tab_mut().search_prev(),
                                    KeyCode::Char('e') => {
                                        if let Some(path) = state.tab().file_path.clone() {
                                            let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                                            disable_raw_mode()?;
                                            execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
                                            let _ = Command::new(&editor)
                                                .arg(&path)
                                                .status();
                                            enable_raw_mode()?;
                                            execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
                                            terminal.clear()?;
                                            if let Ok(new_content) = fs::read_to_string(&path) {
                                                state.tab_mut().content = new_content;
                                            }
                                        }
                                    }
                                    KeyCode::Enter => {
                                        // Try fold toggle first (headings)
                                        let tab = state.tab();
                                        let cursor_idx = tab.visible_line_indices()
                                            .get(tab.cursor)
                                            .copied();
                                        let is_heading = cursor_idx
                                            .and_then(|i| tab.cached_lines.get(i))
                                            .map_or(false, |sl| sl.is_heading);
                                        let link = cursor_idx
                                            .and_then(|i| tab.cached_lines.get(i))
                                            .and_then(|sl| sl.link_url.clone());

                                        if is_heading {
                                            state.tab_mut().toggle_fold();
                                        } else if let Some(url) = link {
                                            if url.ends_with(".md") || url.ends_with(".markdown") {
                                                // Resolve relative to current file's directory
                                                let base_dir = state.tab().file_path.as_ref()
                                                    .and_then(|p| p.parent())
                                                    .map(|p| p.to_path_buf())
                                                    .unwrap_or_else(|| PathBuf::from("."));
                                                let target = base_dir.join(&url);
                                                if let Ok(canonical) = target.canonicalize() {
                                                    if state.open_file(canonical).is_ok() {
                                                        _watcher = rebuild_watcher(&state, file_dirty.clone());
                                                    }
                                                }
                                            } else if url.starts_with("http://") || url.starts_with("https://") {
                                                let _ = Command::new("open").arg(&url).status();
                                            }
                                        }
                                    }
                                    KeyCode::Char('[') => state.tab_mut().fold_all(),
                                    KeyCode::Char(']') => state.tab_mut().unfold_all(),
                                    KeyCode::Char('x') | KeyCode::Char(' ') => {
                                        state.tab_mut().toggle_checkbox();
                                    }
                                    KeyCode::Tab => state.next_tab(),
                                    KeyCode::BackTab => state.prev_tab(),
                                    KeyCode::Char('j') | KeyCode::Down => state.tab_mut().cursor_down(1),
                                    KeyCode::Char('k') | KeyCode::Up => state.tab_mut().cursor_up(1),
                                    KeyCode::Char('f') if ctrl => state.tab_mut().page_down(),
                                    KeyCode::Char('b') if ctrl => state.tab_mut().page_up(),
                                    KeyCode::PageDown => state.tab_mut().page_down(),
                                    KeyCode::PageUp => state.tab_mut().page_up(),
                                    KeyCode::Home | KeyCode::Char('g') => state.tab_mut().cursor_top(),
                                    KeyCode::End | KeyCode::Char('G') => state.tab_mut().cursor_bottom(),
                                    _ => needs_redraw = false,
                                },
                                AppMode::FilePicker => match key.code {
                                    KeyCode::Down => {
                                        state.browser.select_down();
                                        let h = (terminal.size()?.height as usize * 3 / 4).saturating_sub(4);
                                        state.browser.adjust_scroll(h);
                                    }
                                    KeyCode::Up => {
                                        state.browser.select_up();
                                        let h = (terminal.size()?.height as usize * 3 / 4).saturating_sub(4);
                                        state.browser.adjust_scroll(h);
                                    }
                                    KeyCode::Home => {
                                        state.browser.selected = 0;
                                        state.browser.scroll_offset = 0;
                                    }
                                    KeyCode::End => {
                                        let len = state.browser.filtered_indices.len();
                                        if len > 0 {
                                            state.browser.selected = len - 1;
                                        }
                                        let h = (terminal.size()?.height as usize * 3 / 4).saturating_sub(4);
                                        state.browser.adjust_scroll(h);
                                    }
                                    KeyCode::PageDown => {
                                        let h = (terminal.size()?.height as usize * 3 / 4).saturating_sub(4);
                                        let len = state.browser.filtered_indices.len();
                                        if len > 0 {
                                            state.browser.selected = (state.browser.selected + h).min(len - 1);
                                        }
                                        state.browser.adjust_scroll(h);
                                    }
                                    KeyCode::PageUp => {
                                        let h = (terminal.size()?.height as usize * 3 / 4).saturating_sub(4);
                                        state.browser.selected = state.browser.selected.saturating_sub(h);
                                        state.browser.adjust_scroll(h);
                                    }
                                    KeyCode::Enter => {
                                        if let Some(file_path) = state.browser.enter_selected() {
                                            if state.open_file(file_path).is_ok() {
                                                _watcher = rebuild_watcher(&state, file_dirty.clone());
                                                _dir_watcher = setup_dir_watcher(
                                                    &state.browser.current_dir,
                                                    dir_dirty.clone(),
                                                );
                                            }
                                        } else {
                                            // Navigated into a new directory — restart dir watcher
                                            _dir_watcher = setup_dir_watcher(
                                                &state.browser.current_dir,
                                                dir_dirty.clone(),
                                            );
                                        }
                                    }
                                    KeyCode::Esc => {
                                        state.browser.filter.clear();
                                        state.browser.rebuild_filter();
                                        state.mode = AppMode::Reader;
                                    }
                                    KeyCode::Backspace => {
                                        state.browser.filter.pop();
                                        state.browser.rebuild_filter();
                                        state.browser.selected = 0;
                                        state.browser.scroll_offset = 0;
                                    }
                                    KeyCode::Char(c) => {
                                        state.browser.filter.push(c);
                                        state.browser.rebuild_filter();
                                        state.browser.selected = 0;
                                        state.browser.scroll_offset = 0;
                                    }
                                    _ => needs_redraw = false,
                                },
                                AppMode::Search => match key.code {
                                    KeyCode::Esc => {
                                        state.tab_mut().close_search();
                                        state.mode = AppMode::Reader;
                                    }
                                    KeyCode::Enter => {
                                        state.tab_mut().search_first();
                                        state.mode = AppMode::Reader;
                                    }
                                    KeyCode::Backspace => {
                                        state.tab_mut().search_query.pop();
                                        state.tab_mut().update_search();
                                    }
                                    KeyCode::Char(c) => {
                                        state.tab_mut().search_query.push(c);
                                        state.tab_mut().update_search();
                                    }
                                    _ => needs_redraw = false,
                                },
                                AppMode::ThemePicker { .. } => match key.code {
                                    KeyCode::Char('j') | KeyCode::Down => {
                                        let next = (state.theme_index + 1) % state.themes.len();
                                        state.theme_picker_select(next);
                                    }
                                    KeyCode::Char('k') | KeyCode::Up => {
                                        let next = if state.theme_index == 0 {
                                            state.themes.len() - 1
                                        } else {
                                            state.theme_index - 1
                                        };
                                        state.theme_picker_select(next);
                                    }
                                    KeyCode::Home => state.theme_picker_select(0),
                                    KeyCode::End => {
                                        let last = state.themes.len().saturating_sub(1);
                                        state.theme_picker_select(last);
                                    }
                                    KeyCode::Enter => state.theme_picker_confirm(),
                                    KeyCode::Esc => state.theme_picker_cancel(),
                                    _ => needs_redraw = false,
                                },
                                AppMode::FilterPicker { ref mut picker } => match key.code {
                                    KeyCode::Char('j') | KeyCode::Down => picker.select_next(),
                                    KeyCode::Char('k') | KeyCode::Up => picker.select_prev(),
                                    KeyCode::Home => picker.select_first(),
                                    KeyCode::End => picker.select_last(),
                                    KeyCode::Enter => state.label_picker_confirm(),
                                    KeyCode::Esc | KeyCode::Char('l') => state.label_picker_cancel(),
                                    _ => needs_redraw = false,
                                },
                                AppMode::TableOfContents { ref mut picker } => match key.code {
                                    KeyCode::Char('j') | KeyCode::Down => picker.select_next(),
                                    KeyCode::Char('k') | KeyCode::Up => picker.select_prev(),
                                    KeyCode::Home => picker.select_first(),
                                    KeyCode::End => picker.select_last(),
                                    KeyCode::Enter => state.toc_confirm(),
                                    KeyCode::Esc | KeyCode::Char('o') => state.toc_cancel(),
                                    _ => needs_redraw = false,
                                },
                                AppMode::BookmarkList { ref mut picker } => match key.code {
                                    KeyCode::Char('j') | KeyCode::Down => picker.select_next(),
                                    KeyCode::Char('k') | KeyCode::Up => picker.select_prev(),
                                    KeyCode::Home => picker.select_first(),
                                    KeyCode::End => picker.select_last(),
                                    KeyCode::Enter => state.bookmark_list_confirm(),
                                    KeyCode::Esc | KeyCode::Char('B') => state.bookmark_list_cancel(),
                                    _ => needs_redraw = false,
                                },
                                AppMode::Help => match key.code {
                                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter => {
                                        state.close_help();
                                    }
                                    _ => needs_redraw = false,
                                },
                            },
                        }
                    }
                    Event::Mouse(mouse) => {
                        match mouse.kind {
                            MouseEventKind::ScrollDown => {
                                state.tab_mut().scroll_viewport(3, true);
                                needs_redraw = true;
                            }
                            MouseEventKind::ScrollUp => {
                                state.tab_mut().scroll_viewport(3, false);
                                needs_redraw = true;
                            }
                            _ => {}
                        }
                    }
                    Event::Resize(_, _) => needs_redraw = true,
                    _ => {}
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
    Ok(())
}
