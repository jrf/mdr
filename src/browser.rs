use std::path::PathBuf;

pub struct BrowserEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub struct BrowserState {
    pub current_dir: PathBuf,
    pub entries: Vec<BrowserEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
}

impl BrowserState {
    pub fn new(dir: PathBuf) -> Self {
        let mut state = Self {
            current_dir: dir,
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
        };
        state.load_dir();
        state
    }

    pub fn load_dir(&mut self) {
        self.entries.clear();

        // Add parent directory entry
        if let Some(parent) = self.current_dir.parent() {
            self.entries.push(BrowserEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
            });
        }

        let Ok(read_dir) = std::fs::read_dir(&self.current_dir) else {
            return;
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in read_dir.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files
            if name.starts_with('.') {
                continue;
            }

            if file_type.is_dir() {
                dirs.push(BrowserEntry {
                    name: format!("{}/", name),
                    path: entry.path(),
                    is_dir: true,
                });
            } else if name.ends_with(".md") || name.ends_with(".markdown") {
                files.push(BrowserEntry {
                    name,
                    path: entry.path(),
                    is_dir: false,
                });
            }
        }

        dirs.sort_by(|a, b| a.name.cmp(&b.name));
        files.sort_by(|a, b| a.name.cmp(&b.name));

        self.entries.extend(dirs);
        self.entries.extend(files);
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn select_down(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1).min(self.entries.len() - 1);
        }
    }

    pub fn select_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_down_n(&mut self, n: usize) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + n).min(self.entries.len() - 1);
        }
    }

    pub fn select_up_n(&mut self, n: usize) {
        self.selected = self.selected.saturating_sub(n);
    }

    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    pub fn select_last(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
    }

    /// Returns Some(path) if a markdown file was selected, None if navigated into a directory.
    pub fn enter_selected(&mut self) -> Option<PathBuf> {
        let entry = self.entries.get(self.selected)?;
        if entry.is_dir {
            self.current_dir = entry.path.clone();
            self.load_dir();
            None
        } else {
            Some(entry.path.clone())
        }
    }

    pub fn adjust_scroll(&mut self, visible_height: usize) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected - visible_height + 1;
        }
    }
}
