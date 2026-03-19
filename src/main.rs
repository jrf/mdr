mod browser;
mod markdown;
mod state;
mod theme;
mod ui;

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use state::{AppMode, AppState};

fn setup_watcher(path: &PathBuf, flag: Arc<AtomicBool>) -> Option<RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            if event.kind.is_modify() {
                flag.store(true, Ordering::Relaxed);
            }
        }
    })
    .ok()?;

    watcher
        .watch(
            path.parent().unwrap_or(path),
            RecursiveMode::NonRecursive,
        )
        .ok()?;

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

fn main() -> io::Result<()> {
    let file_arg = env::args().nth(1);

    let mut state = if let Some(ref arg) = file_arg {
        let file_path = PathBuf::from(arg).canonicalize().map_err(|e| {
            eprintln!("error: {}: {}", arg, e);
            e
        })?;
        let content = fs::read_to_string(&file_path)?;
        AppState::new_reader(file_path, content)
    } else {
        let dir = env::current_dir()?;
        AppState::new_picker(dir)
    };

    // File change flag (set by watcher, cleared by main loop)
    let file_dirty = Arc::new(AtomicBool::new(false));
    let dir_dirty = Arc::new(AtomicBool::new(false));

    // Set up watcher if we started with a file
    let mut _watcher: Option<RecommendedWatcher> = state
        .file_path
        .as_ref()
        .and_then(|p| setup_watcher(p, file_dirty.clone()));

    // Set up directory watcher for the file picker
    let mut _dir_watcher: Option<RecommendedWatcher> =
        Some(setup_dir_watcher(&state.browser.current_dir, dir_dirty.clone())).flatten();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let mut needs_redraw = true;
    loop {
        if needs_redraw {
            terminal.draw(|f| ui::draw(f, &mut state))?;
            needs_redraw = false;
        }

        // Check for file changes (only in reader mode)
        if matches!(state.mode, AppMode::Reader) && file_dirty.swap(false, Ordering::Relaxed) {
            if let Some(ref path) = state.file_path {
                if let Ok(new_content) = fs::read_to_string(path) {
                    if new_content != state.content {
                        state.content = new_content;
                        needs_redraw = true;
                    }
                }
            }
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
                        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('c') if ctrl => break,
                            _ => match state.mode {
                                AppMode::Reader => match key.code {
                                    KeyCode::Esc if !state.search_query.is_empty() => {
                                        state.search_query.clear();
                                        state.search_matches.clear();
                                        state.search_current = 0;
                                    }
                                    KeyCode::Char('f') if !ctrl => {
                                        state.browser.filter.clear();
                                        state.browser.rebuild_filter();
                                        state.mode = AppMode::FilePicker;
                                    }
                                    KeyCode::Char('t') => state.open_theme_picker(),
                                    KeyCode::Char('?') => state.open_help(),
                                    KeyCode::Char('/') => state.open_search(),
                                    KeyCode::Char('n') => state.search_next(),
                                    KeyCode::Char('N') => state.search_prev(),
                                    KeyCode::Char('e') => {
                                        if let Some(ref path) = state.file_path {
                                            let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                                            disable_raw_mode()?;
                                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                                            let _ = Command::new(&editor)
                                                .arg(path)
                                                .status();
                                            enable_raw_mode()?;
                                            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                            terminal.clear()?;
                                            if let Ok(new_content) = fs::read_to_string(path) {
                                                state.content = new_content;
                                            }
                                        }
                                    }
                                    KeyCode::Char('j') | KeyCode::Down => state.scroll_down(1),
                                    KeyCode::Char('k') | KeyCode::Up => state.scroll_up(1),
                                    KeyCode::Char('f') if ctrl => {
                                        let h = terminal.size()?.height.saturating_sub(6) as usize;
                                        state.scroll_down(h);
                                    }
                                    KeyCode::Char('b') if ctrl => {
                                        let h = terminal.size()?.height.saturating_sub(6) as usize;
                                        state.scroll_up(h);
                                    }
                                    KeyCode::PageDown => {
                                        let h = terminal.size()?.height.saturating_sub(6) as usize;
                                        state.scroll_down(h);
                                    }
                                    KeyCode::PageUp => {
                                        let h = terminal.size()?.height.saturating_sub(6) as usize;
                                        state.scroll_up(h);
                                    }
                                    KeyCode::Home | KeyCode::Char('g') => state.scroll_top(),
                                    KeyCode::End | KeyCode::Char('G') => state.scroll_bottom(),
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
                                                _watcher = state
                                                    .file_path
                                                    .as_ref()
                                                    .and_then(|p| setup_watcher(p, file_dirty.clone()));
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
                                    KeyCode::Esc => state.close_search(),
                                    KeyCode::Enter => {
                                        state.search_first();
                                        state.mode = AppMode::Reader;
                                    }
                                    KeyCode::Backspace => {
                                        state.search_query.pop();
                                        state.update_search();
                                    }
                                    KeyCode::Char(c) => {
                                        state.search_query.push(c);
                                        state.update_search();
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
                    Event::Resize(_, _) => needs_redraw = true,
                    _ => {}
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
