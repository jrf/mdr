use std::fs;
use std::io;
use std::path::PathBuf;

use crate::browser::BrowserState;
use crate::theme::{default_theme, Theme, ALL_THEMES};

pub enum AppMode {
    Browser,
    Reader,
    ThemePicker { previous_mode: PreviousMode, original_index: usize },
    Help { previous_mode: PreviousMode },
}

#[derive(Clone, Copy)]
pub enum PreviousMode {
    Browser,
    Reader,
}

pub struct AppState {
    pub mode: AppMode,
    pub content: String,
    pub file_path: Option<PathBuf>,
    pub scroll: usize,
    pub theme: Theme,
    pub theme_index: usize,
    pub browser: BrowserState,
}

impl AppState {
    pub fn new_browser(dir: PathBuf) -> Self {
        Self {
            mode: AppMode::Browser,
            content: String::new(),
            file_path: None,
            scroll: 0,
            theme: default_theme(),
            theme_index: 5,
            browser: BrowserState::new(dir),
        }
    }

    pub fn new_reader(file_path: PathBuf, content: String) -> Self {
        let browser_dir = file_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            mode: AppMode::Reader,
            content,
            file_path: Some(file_path),
            scroll: 0,
            theme: default_theme(),
            theme_index: 5,
            browser: BrowserState::new(browser_dir),
        }
    }

    pub fn open_file(&mut self, path: PathBuf) -> io::Result<()> {
        let content = fs::read_to_string(&path)?;
        self.content = content;
        self.file_path = Some(path);
        self.scroll = 0;
        self.mode = AppMode::Reader;
        Ok(())
    }

    pub fn back_to_browser(&mut self) {
        self.mode = AppMode::Browser;
    }

    pub fn open_theme_picker(&mut self) {
        let prev = match self.mode {
            AppMode::Browser => PreviousMode::Browser,
            AppMode::Reader => PreviousMode::Reader,
            AppMode::ThemePicker { .. } => return,
            AppMode::Help { .. } => return,
        };
        self.mode = AppMode::ThemePicker {
            previous_mode: prev,
            original_index: self.theme_index,
        };
    }

    pub fn theme_picker_select(&mut self, index: usize) {
        self.theme_index = index;
        self.theme = ALL_THEMES[index].1;
    }

    pub fn theme_picker_confirm(&mut self) {
        if let AppMode::ThemePicker { previous_mode, .. } = self.mode {
            self.mode = match previous_mode {
                PreviousMode::Browser => AppMode::Browser,
                PreviousMode::Reader => AppMode::Reader,
            };
        }
    }

    pub fn theme_picker_cancel(&mut self) {
        if let AppMode::ThemePicker { previous_mode, original_index } = self.mode {
            self.theme_index = original_index;
            self.theme = ALL_THEMES[original_index].1;
            self.mode = match previous_mode {
                PreviousMode::Browser => AppMode::Browser,
                PreviousMode::Reader => AppMode::Reader,
            };
        }
    }

    pub fn open_help(&mut self) {
        let prev = match self.mode {
            AppMode::Browser => PreviousMode::Browser,
            AppMode::Reader => PreviousMode::Reader,
            AppMode::ThemePicker { .. } => return,
            AppMode::Help { .. } => return,
        };
        self.mode = AppMode::Help { previous_mode: prev };
    }

    pub fn close_help(&mut self) {
        if let AppMode::Help { previous_mode } = self.mode {
            self.mode = match previous_mode {
                PreviousMode::Browser => AppMode::Browser,
                PreviousMode::Reader => AppMode::Reader,
            };
        }
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_add(n);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub fn scroll_top(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_bottom(&mut self) {
        self.scroll = usize::MAX;
    }
}
