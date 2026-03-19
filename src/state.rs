use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::PathBuf;

use crate::browser::BrowserState;
use crate::markdown::StyledLine;
use crate::theme::{default_theme, Theme, ALL_THEMES};

pub enum AppMode {
    Reader,
    Search,
    FilePicker,
    ThemePicker { original_index: usize },
    Help,
}

pub struct AppState {
    pub mode: AppMode,
    pub content: String,
    pub file_path: Option<PathBuf>,
    pub scroll: usize,
    pub total_lines: usize,
    pub visible_height: usize,
    pub theme: Theme,
    pub theme_index: usize,
    pub browser: BrowserState,
    pub cursor: usize,
    pub search_query: String,
    pub search_matches: Vec<usize>,
    pub search_current: usize,
    // Parsed markdown cache — invalidated when content, theme, or width changes
    pub cached_lines: Vec<StyledLine<'static>>,
    pub cache_content_hash: u64,
    pub cache_theme: Theme,
    pub cache_width: u16,
}

impl AppState {
    pub fn new_picker(dir: PathBuf) -> Self {
        Self {
            mode: AppMode::FilePicker,
            content: String::new(),
            file_path: None,
            scroll: 0,
            total_lines: 0,
            visible_height: 0,
            theme: default_theme(),
            theme_index: 5,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_current: 0,
            cursor: 0,
            browser: BrowserState::new(dir),
            cached_lines: Vec::new(),
            cache_content_hash: 0,
            cache_theme: default_theme(),
            cache_width: 0,
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
            total_lines: 0,
            visible_height: 0,
            theme: default_theme(),
            theme_index: 5,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_current: 0,
            cursor: 0,
            browser: BrowserState::new(browser_dir),
            cached_lines: Vec::new(),
            cache_content_hash: 0,
            cache_theme: default_theme(),
            cache_width: 0,
        }
    }

    pub fn open_file(&mut self, path: PathBuf) -> io::Result<()> {
        let content = fs::read_to_string(&path)?;
        self.content = content;
        self.file_path = Some(path);
        self.scroll = 0;
        self.cursor = 0;
        self.mode = AppMode::Reader;
        Ok(())
    }

    fn content_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.content.hash(&mut hasher);
        hasher.finish()
    }

    pub fn get_parsed_lines(&mut self, width: u16) -> &[StyledLine<'static>] {
        let hash = self.content_hash();
        if hash != self.cache_content_hash || self.theme != self.cache_theme || width != self.cache_width {
            self.cached_lines = crate::markdown::parse_markdown(&self.content, self.theme, width);
            self.cache_content_hash = hash;
            self.cache_theme = self.theme;
            self.cache_width = width;
        }
        &self.cached_lines
    }

    pub fn open_theme_picker(&mut self) {
        match self.mode {
            AppMode::ThemePicker { .. } | AppMode::Help => return,
            _ => {}
        }
        self.mode = AppMode::ThemePicker {
            original_index: self.theme_index,
        };
    }

    pub fn theme_picker_select(&mut self, index: usize) {
        self.theme_index = index;
        self.theme = ALL_THEMES[index].1;
    }

    pub fn theme_picker_confirm(&mut self) {
        if matches!(self.mode, AppMode::ThemePicker { .. }) {
            self.mode = AppMode::Reader;
        }
    }

    pub fn theme_picker_cancel(&mut self) {
        if let AppMode::ThemePicker { original_index } = self.mode {
            self.theme_index = original_index;
            self.theme = ALL_THEMES[original_index].1;
            self.mode = AppMode::Reader;
        }
    }

    pub fn open_help(&mut self) {
        match self.mode {
            AppMode::ThemePicker { .. } | AppMode::Help => return,
            _ => {}
        }
        self.mode = AppMode::Help;
    }

    pub fn close_help(&mut self) {
        if matches!(self.mode, AppMode::Help) {
            self.mode = AppMode::Reader;
        }
    }

    pub fn cursor_down(&mut self, n: usize) {
        let max = self.total_lines.saturating_sub(1);
        self.cursor = self.cursor.saturating_add(n).min(max);
        self.ensure_cursor_visible();
    }

    pub fn cursor_up(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
        self.ensure_cursor_visible();
    }

    pub fn cursor_top(&mut self) {
        self.cursor = 0;
        self.scroll = 0;
    }

    pub fn cursor_bottom(&mut self) {
        self.cursor = self.total_lines.saturating_sub(1);
        self.scroll = self.total_lines.saturating_sub(self.visible_height);
    }

    fn ensure_cursor_visible(&mut self) {
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + self.visible_height {
            self.scroll = self.cursor.saturating_sub(self.visible_height - 1);
        }
    }

    /// Toggle the checkbox on the line under the cursor.
    /// Returns true if a toggle was performed.
    pub fn toggle_checkbox(&mut self) -> bool {
        let source_line = match self.cached_lines.get(self.cursor) {
            Some(sl) => match sl.source_line {
                Some(n) => n,
                None => return false,
            },
            None => return false,
        };

        // Find and toggle the checkbox pattern on this source line
        let mut content_lines: Vec<String> = self.content.lines().map(String::from).collect();
        if source_line >= content_lines.len() {
            return false;
        }

        let line = &content_lines[source_line];
        let toggled = if let Some(pos) = line.find("- [ ] ") {
            let mut new = line.clone();
            new.replace_range(pos..pos + 6, "- [x] ");
            new
        } else if let Some(pos) = line.find("- [x] ") {
            let mut new = line.clone();
            new.replace_range(pos..pos + 6, "- [ ] ");
            new
        } else if let Some(pos) = line.find("- [X] ") {
            let mut new = line.clone();
            new.replace_range(pos..pos + 6, "- [ ] ");
            new
        } else {
            return false;
        };

        content_lines[source_line] = toggled;
        let had_trailing_newline = self.content.ends_with('\n');
        self.content = content_lines.join("\n");
        if had_trailing_newline {
            self.content.push('\n');
        }

        // Write back to file
        if let Some(ref path) = self.file_path {
            let _ = fs::write(path, &self.content);
        }

        // Invalidate cache
        self.cache_content_hash = 0;

        true
    }

    pub fn open_search(&mut self) {
        self.search_query.clear();
        self.search_matches.clear();
        self.search_current = 0;
        self.mode = AppMode::Search;
    }

    pub fn close_search(&mut self) {
        self.search_query.clear();
        self.search_matches.clear();
        self.search_current = 0;
        self.mode = AppMode::Reader;
    }

    pub fn update_search(&mut self) {
        self.search_matches.clear();
        self.search_current = 0;
        if self.search_query.is_empty() {
            return;
        }
        let query_lower = self.search_query.to_lowercase();
        for (i, line) in self.content.lines().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                self.search_matches.push(i);
            }
        }
    }

    pub fn search_next(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_current = (self.search_current + 1) % self.search_matches.len();
        self.scroll_to_match();
    }

    pub fn search_prev(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.search_current == 0 {
            self.search_current = self.search_matches.len() - 1;
        } else {
            self.search_current -= 1;
        }
        self.scroll_to_match();
    }

    pub fn search_first(&mut self) {
        if !self.search_matches.is_empty() {
            if let Some(idx) = self.search_matches.iter().position(|&l| l >= self.scroll) {
                self.search_current = idx;
            } else {
                self.search_current = 0;
            }
            self.scroll_to_match();
        }
    }

    fn scroll_to_match(&mut self) {
        if let Some(&line) = self.search_matches.get(self.search_current) {
            self.cursor = line;
            self.scroll = line.saturating_sub(self.visible_height / 3);
        }
    }
}
