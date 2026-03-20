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
    let initial_theme = cfg.theme.as_deref()
        .and_then(|name| theme::find_theme(name))
        .map(|(idx, _)| idx)
        .unwrap_or(5);

    let mut state = if let Some(ref arg) = file_arg {
        let file_path = PathBuf::from(arg).canonicalize().map_err(|e| {
            eprintln!("error: {}: {}", arg, e);
            e
        })?;
        let content = fs::read_to_string(&file_path)?;
        AppState::new_reader(file_path, content, initial_theme, cfg.scrollbar)
    } else {
        let dir = env::current_dir()?;
        AppState::new_picker(dir, initial_theme, cfg.scrollbar)
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
                            KeyCode::Char('q') => break,
                            KeyCode::Char('c') if ctrl => break,
                            _ => match state.mode {
                                AppMode::Reader => match key.code {
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
                                    KeyCode::Char('F') => state.tab_mut().toggle_filter_tasks(),
                                    KeyCode::Char('t') => state.open_theme_picker(),
                                    KeyCode::Char('?') => state.open_help(),
                                    KeyCode::Char('/') => {
                                        state.tab_mut().open_search();
                                        state.mode = AppMode::Search;
                                    }
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
                                    KeyCode::Enter => state.tab_mut().toggle_fold(),
                                    KeyCode::Char('x') | KeyCode::Char(' ') => {
                                        state.tab_mut().toggle_checkbox();
                                    }
                                    KeyCode::Tab => state.tab_mut().next_task(),
                                    KeyCode::BackTab => state.tab_mut().prev_task(),
                                    KeyCode::Char('j') | KeyCode::Down => state.tab_mut().cursor_down(1),
                                    KeyCode::Char('k') | KeyCode::Up => state.tab_mut().cursor_up(1),
                                    KeyCode::Char('f') if ctrl => {
                                        let h = terminal.size()?.height.saturating_sub(6) as usize;
                                        state.tab_mut().cursor_down(h);
                                    }
                                    KeyCode::Char('b') if ctrl => {
                                        let h = terminal.size()?.height.saturating_sub(6) as usize;
                                        state.tab_mut().cursor_up(h);
                                    }
                                    KeyCode::PageDown => {
                                        let h = terminal.size()?.height.saturating_sub(6) as usize;
                                        state.tab_mut().cursor_down(h);
                                    }
                                    KeyCode::PageUp => {
                                        let h = terminal.size()?.height.saturating_sub(6) as usize;
                                        state.tab_mut().cursor_up(h);
                                    }
                                    KeyCode::Home | KeyCode::Char('g') => state.tab_mut().cursor_top(),
                                    KeyCode::End | KeyCode::Char('G') => state.tab_mut().cursor_bottom(),
                                    KeyCode::Char('H') => state.prev_tab(),
                                    KeyCode::Char('L') => state.next_tab(),
                                    KeyCode::Char('W') => state.close_tab(),
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
                                        let next = (state.theme_index + 1) % theme::ALL_THEMES.len();
                                        state.theme_picker_select(next);
                                    }
                                    KeyCode::Char('k') | KeyCode::Up => {
                                        let next = if state.theme_index == 0 {
                                            theme::ALL_THEMES.len() - 1
                                        } else {
                                            state.theme_index - 1
                                        };
                                        state.theme_picker_select(next);
                                    }
                                    KeyCode::Enter => state.theme_picker_confirm(),
                                    KeyCode::Esc => state.theme_picker_cancel(),
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
