use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::theme::ThemeConfig;

const EMBEDDED_THEMES: &[(&str, &str)] = &[
    ("catppuccin frappe", include_str!("../themes/catppuccin-frappe.toml")),
    ("catppuccin latte", include_str!("../themes/catppuccin-latte.toml")),
    ("catppuccin macchiato", include_str!("../themes/catppuccin-macchiato.toml")),
    ("catppuccin mocha", include_str!("../themes/catppuccin-mocha.toml")),
    ("classic", include_str!("../themes/classic.toml")),
    ("fire", include_str!("../themes/fire.toml")),
    ("matrix", include_str!("../themes/matrix.toml")),
    ("monochrome", include_str!("../themes/monochrome.toml")),
    ("ocean", include_str!("../themes/ocean.toml")),
    ("purple", include_str!("../themes/purple.toml")),
    ("sunset", include_str!("../themes/sunset.toml")),
    ("synthwave", include_str!("../themes/synthwave.toml")),
    ("tokyo night", include_str!("../themes/tokyo-night.toml")),
    ("tokyo night day", include_str!("../themes/tokyo-night-day.toml")),
    ("tokyo night moon", include_str!("../themes/tokyo-night-moon.toml")),
    ("tokyo night storm", include_str!("../themes/tokyo-night-storm.toml")),
];

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

fn embedded_theme_configs() -> BTreeMap<String, ThemeConfig> {
    EMBEDDED_THEMES
        .iter()
        .map(|(name, contents)| {
            let config = toml::from_str(contents).expect("embedded theme must be valid TOML");
            ((*name).to_string(), config)
        })
        .collect()
}

/// Load built-in themes, then overlay ~/.config/mdr/themes/*.toml.
/// User theme names are derived from filenames and override matching built-ins.
pub fn load_theme_configs() -> BTreeMap<String, ThemeConfig> {
    let mut themes = embedded_theme_configs();

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

#[cfg(test)]
mod tests {
    use super::embedded_theme_configs;

    #[test]
    fn embeds_every_built_in_theme() {
        let themes = embedded_theme_configs();
        let expected = [
            "catppuccin frappe",
            "catppuccin latte",
            "catppuccin macchiato",
            "catppuccin mocha",
            "classic",
            "fire",
            "matrix",
            "monochrome",
            "ocean",
            "purple",
            "sunset",
            "synthwave",
            "tokyo night",
            "tokyo night day",
            "tokyo night moon",
            "tokyo night storm",
        ];

        assert_eq!(themes.len(), expected.len());
        for name in expected {
            assert!(themes.contains_key(name), "missing embedded theme: {name}");
        }
    }
}
