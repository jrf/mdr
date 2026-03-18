mod markdown;
mod state;
mod theme;
mod ui;

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
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

use state::AppState;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: meld <file.md>");
        std::process::exit(1);
    }

    let file_path = PathBuf::from(&args[1]).canonicalize().map_err(|e| {
        eprintln!("error: {}: {}", args[1], e);
        e
    })?;

    let content = fs::read_to_string(&file_path)?;
    let mut state = AppState::new(Some(file_path.clone()), content);

    // File change flag (set by watcher, cleared by main loop)
    let file_dirty = Arc::new(AtomicBool::new(false));

    // File watcher
    let flag = file_dirty.clone();
    let watch_path = file_path.clone();
    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    flag.store(true, Ordering::Relaxed);
                }
            }
        })
        .expect("failed to create file watcher");

    watcher
        .watch(
            watch_path.parent().unwrap_or(&watch_path),
            RecursiveMode::NonRecursive,
        )
        .expect("failed to watch file");

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
            terminal.draw(|f| ui::draw(f, &state))?;
            needs_redraw = false;
        }

        // Check for file changes (coalesces all watcher events automatically)
        if file_dirty.swap(false, Ordering::Relaxed) {
            if let Ok(new_content) = fs::read_to_string(&file_path) {
                if new_content != state.content {
                    state.content = new_content;
                    needs_redraw = true;
                }
            }
        }

        // Poll for terminal events
        if event::poll(Duration::from_millis(50))? {
            if let Ok(ev) = event::read() {
                match ev {
                    Event::Key(key) => {
                        needs_redraw = true;
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('c')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                break
                            }
                            KeyCode::Char('j') | KeyCode::Down => state.scroll_down(1),
                            KeyCode::Char('k') | KeyCode::Up => state.scroll_up(1),
                            KeyCode::Char('d')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                state.scroll_down(20)
                            }
                            KeyCode::Char('u')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                state.scroll_up(20)
                            }
                            KeyCode::Char('g') => state.scroll_top(),
                            KeyCode::Char('G') => state.scroll_bottom(),
                            KeyCode::Char('t') => state.cycle_theme(),
                            _ => needs_redraw = false,
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
