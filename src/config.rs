use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::theme::ThemeConfig;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default = "default_scrollbar")]
    pub scrollbar: bool,
}

fn default_scrollbar() -> bool {
    true
}

fn config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join(".config").join("mdr"))
}

fn config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

fn themes_dir() -> Option<PathBuf> {
    config_dir().map(|d| d.join("themes"))
}

pub fn load_config() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => return Config { scrollbar: true, ..Default::default() },
    };
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Config { scrollbar: true, ..Default::default() },
    };

    toml::from_str(&contents).unwrap_or(Config { scrollbar: true, ..Default::default() })
}

/// Load all theme files from ~/.config/mdr/themes/*.toml.
/// Theme name is derived from filename (minus .toml extension).
pub fn load_theme_configs() -> BTreeMap<String, ThemeConfig> {
    let mut themes = BTreeMap::new();

    let dir = match themes_dir() {
        Some(d) => d,
        None => return themes,
    };

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return themes,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.replace('-', " "),
            None => continue,
        };
        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Ok(cfg) = toml::from_str::<ThemeConfig>(&contents) {
            themes.insert(name, cfg);
        }
    }

    themes
}
