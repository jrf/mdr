use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::PathBuf;

use crate::browser::BrowserState;
use crate::markdown::StyledLine;
use crate::theme::{Theme, ALL_THEMES};

pub enum AppMode {
    Reader,
    Search,
    FilePicker,
    ThemePicker { original_index: usize },
    Help,
}

pub struct Tab {
    pub content: String,
    pub file_path: Option<PathBuf>,
    pub scroll: usize,
    pub cursor: usize,
    pub total_lines: usize,
    pub visible_height: usize,
    pub file_updated: bool,
    pub filter_tasks: bool,
    pub folded_headings: HashSet<String>,
    pub search_query: String,
    pub search_matches: Vec<usize>,
    pub search_current: usize,
    pub cached_lines: Vec<StyledLine<'static>>,
    pub cache_content_hash: u64,
    pub cache_theme: Theme,
    pub cache_width: u16,
    pub cache_filter: bool,
}

impl Tab {
    pub fn new(file_path: PathBuf, content: String, theme: Theme) -> Self {
        Self {
            content,
            file_path: Some(file_path),
            scroll: 0,
            cursor: 0,
            total_lines: 0,
            visible_height: 0,
            file_updated: false,
            filter_tasks: false,
            folded_headings: HashSet::new(),
            search_query: String::new(),
            search_matches: Vec::new(),
            search_current: 0,
            cached_lines: Vec::new(),
            cache_content_hash: 0,
            cache_theme: theme,
            cache_width: 0,
            cache_filter: false,
        }
    }

    fn empty(theme: Theme) -> Self {
        Self {
            content: String::new(),
            file_path: None,
            scroll: 0,
            cursor: 0,
            total_lines: 0,
            visible_height: 0,
            file_updated: false,
            filter_tasks: false,
            folded_headings: HashSet::new(),
            search_query: String::new(),
            search_matches: Vec::new(),
            search_current: 0,
            cached_lines: Vec::new(),
            cache_content_hash: 0,
            cache_theme: theme,
            cache_width: 0,
            cache_filter: false,
        }
    }

    fn content_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.content.hash(&mut hasher);
        hasher.finish()
    }

    pub fn get_parsed_lines(&mut self, width: u16, theme: Theme) -> &[StyledLine<'static>] {
        let hash = self.content_hash();
        if hash != self.cache_content_hash
            || theme != self.cache_theme
            || width != self.cache_width
            || self.filter_tasks != self.cache_filter
        {
            let mut lines = crate::markdown::parse_markdown(&self.content, theme, width);
            if self.filter_tasks {
                lines = filter_task_lines(lines);
            }
            self.cached_lines = lines;
            self.cache_content_hash = hash;
            self.cache_theme = theme;
            self.cache_width = width;
            self.cache_filter = self.filter_tasks;
        }
        &self.cached_lines
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

    pub fn scroll_viewport(&mut self, n: usize, down: bool) {
        let max_scroll = self.total_lines.saturating_sub(self.visible_height);
        if down {
            self.scroll = self.scroll.saturating_add(n).min(max_scroll);
        } else {
            self.scroll = self.scroll.saturating_sub(n);
        }
        if self.cursor < self.scroll {
            self.cursor = self.scroll;
        } else if self.cursor >= self.scroll + self.visible_height {
            self.cursor = self.scroll + self.visible_height - 1;
        }
    }

    fn ensure_cursor_visible(&mut self) {
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + self.visible_height {
            self.scroll = self.cursor.saturating_sub(self.visible_height - 1);
        }
    }

    pub fn toggle_checkbox(&mut self) -> bool {
        let idx = match self.cursor_line_idx() {
            Some(i) => i,
            None => return false,
        };
        let source_line = match self.cached_lines.get(idx) {
            Some(sl) => match sl.source_line {
                Some(n) => n,
                None => return false,
            },
            None => return false,
        };

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

        if let Some(ref path) = self.file_path {
            let _ = fs::write(path, &self.content);
        }

        self.cache_content_hash = 0;
        true
    }

    pub fn next_task(&mut self) {
        let indices = self.visible_line_indices();
        let len = indices.len();
        if len == 0 {
            return;
        }
        let start = self.cursor + 1;
        for i in 0..len {
            let display_idx = (start + i) % len;
            let line_idx = indices[display_idx];
            if let Some(sl) = self.cached_lines.get(line_idx) {
                if let Some(src) = sl.source_line {
                    let line = self.content.lines().nth(src).unwrap_or("");
                    if line.contains("- [ ] ") {
                        self.cursor = display_idx;
                        self.ensure_cursor_visible();
                        return;
                    }
                }
            }
        }
    }

    pub fn prev_task(&mut self) {
        let indices = self.visible_line_indices();
        let len = indices.len();
        if len == 0 {
            return;
        }
        for i in 1..=len {
            let display_idx = (self.cursor + len - i) % len;
            let line_idx = indices[display_idx];
            if let Some(sl) = self.cached_lines.get(line_idx) {
                if let Some(src) = sl.source_line {
                    let line = self.content.lines().nth(src).unwrap_or("");
                    if line.contains("- [ ] ") {
                        self.cursor = display_idx;
                        self.ensure_cursor_visible();
                        return;
                    }
                }
            }
        }
    }

    fn cursor_line_idx(&self) -> Option<usize> {
        let indices = self.visible_line_indices();
        indices.get(self.cursor).copied()
    }

    pub fn toggle_fold(&mut self) {
        if let Some(idx) = self.cursor_line_idx() {
            if let Some(sl) = self.cached_lines.get(idx) {
                if let Some(ref text) = sl.heading_text {
                    if !self.folded_headings.remove(text) {
                        self.folded_headings.insert(text.clone());
                    }
                }
            }
        }
    }

    pub fn visible_line_indices(&self) -> Vec<usize> {
        if self.folded_headings.is_empty() {
            return (0..self.cached_lines.len()).collect();
        }

        let mut indices = Vec::new();
        let mut skip_until_level: Option<u8> = None;
        let mut kept_blank_after_fold = false;

        for (i, sl) in self.cached_lines.iter().enumerate() {
            if let Some(level) = sl.heading_level {
                if let Some(fold_level) = skip_until_level {
                    if level <= fold_level {
                        skip_until_level = None;
                        kept_blank_after_fold = false;
                    } else {
                        continue;
                    }
                }
                indices.push(i);
                if let Some(ref text) = sl.heading_text {
                    if self.folded_headings.contains(text) {
                        skip_until_level = Some(level);
                        kept_blank_after_fold = false;
                    }
                }
            } else if skip_until_level.is_some() {
                if sl.is_blank && !kept_blank_after_fold {
                    indices.push(i);
                    kept_blank_after_fold = true;
                }
            } else {
                indices.push(i);
            }
        }

        indices
    }

    pub fn toggle_filter_tasks(&mut self) {
        self.filter_tasks = !self.filter_tasks;
        self.cursor = 0;
        self.scroll = 0;
    }

    pub fn open_search(&mut self) {
        self.search_query.clear();
        self.search_matches.clear();
        self.search_current = 0;
    }

    pub fn close_search(&mut self) {
        self.search_query.clear();
        self.search_matches.clear();
        self.search_current = 0;
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

pub struct AppState {
    pub mode: AppMode,
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub theme: Theme,
    pub theme_index: usize,
    pub browser: BrowserState,
    pub scrollbar: bool,
}

impl AppState {
    pub fn new_picker(dir: PathBuf, theme_index: usize, scrollbar: bool) -> Self {
        let theme = ALL_THEMES[theme_index].1;
        Self {
            mode: AppMode::FilePicker,
            tabs: vec![Tab::empty(theme)],
            active_tab: 0,
            theme,
            theme_index,
            browser: BrowserState::new(dir),
            scrollbar,
        }
    }

    pub fn new_reader(file_path: PathBuf, content: String, theme_index: usize, scrollbar: bool) -> Self {
        let theme = ALL_THEMES[theme_index].1;
        let browser_dir = file_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            mode: AppMode::Reader,
            tabs: vec![Tab::new(file_path, content, theme)],
            active_tab: 0,
            theme,
            theme_index,
            browser: BrowserState::new(browser_dir),
            scrollbar,
        }
    }

    /// Access the active tab.
    pub fn tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    /// Access the active tab mutably.
    pub fn tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    /// Open a file in a new tab, or switch to it if already open.
    pub fn open_file(&mut self, path: PathBuf) -> io::Result<()> {
        // Check if already open
        if let Some(idx) = self.tabs.iter().position(|t| t.file_path.as_ref() == Some(&path)) {
            self.active_tab = idx;
            self.mode = AppMode::Reader;
            return Ok(());
        }
        let content = fs::read_to_string(&path)?;
        // Replace the placeholder tab (empty, no file) if it's the only one
        if self.tabs.len() == 1 && self.tabs[0].file_path.is_none() {
            self.tabs[0] = Tab::new(path, content, self.theme);
            self.active_tab = 0;
        } else {
            let tab = Tab::new(path, content, self.theme);
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
        }
        self.mode = AppMode::Reader;
        Ok(())
    }

    pub fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if self.tabs.len() > 1 {
            if self.active_tab == 0 {
                self.active_tab = self.tabs.len() - 1;
            } else {
                self.active_tab -= 1;
            }
        }
    }

    pub fn close_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.active_tab);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    /// Collect all file paths from open tabs (for the file watcher).
    pub fn tab_file_paths(&self) -> Vec<PathBuf> {
        self.tabs
            .iter()
            .filter_map(|t| t.file_path.clone())
            .collect()
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
            let mut cfg = crate::config::load_config();
            cfg.theme = Some(ALL_THEMES[self.theme_index].0.to_string());
            crate::config::save_config(&cfg);
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
}

/// Filter parsed lines to only unchecked tasks and their heading context.
/// All headings are shown. A blank line separates each section.
fn filter_task_lines(lines: Vec<StyledLine<'static>>) -> Vec<StyledLine<'static>> {
    let mut result: Vec<StyledLine<'static>> = Vec::new();

    for sl in lines {
        if sl.is_heading {
            // Ensure a blank line before each heading (except the first)
            if !result.is_empty() && !result.last().map_or(true, |l| l.is_blank) {
                result.push(StyledLine {
                    line: ratatui::text::Line::default(),
                    is_blank: true,
                    is_heading: false,
                    heading_level: None,
                    heading_text: None,
                    source_line: None,
                });
            }
            result.push(sl);
            // Blank line after heading
            result.push(StyledLine {
                line: ratatui::text::Line::default(),
                is_blank: true,
                is_heading: false,
                heading_level: None,
                heading_text: None,
                source_line: None,
            });
        } else if sl.source_line.is_some() {
            // Check the marker span (first span) for unchecked status
            let is_unchecked = sl.line.spans.first()
                .map_or(false, |s| s.content.contains("[ ]"));
            if is_unchecked {
                result.push(sl);
            }
        }
    }

    // Remove trailing blank lines
    while result.last().map_or(false, |l| l.is_blank) {
        result.pop();
    }

    result
}
