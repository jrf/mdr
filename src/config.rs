use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::theme::ThemeConfig;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub theme_catalog: Option<String>,
    #[serde(default = "default_scrollbar")]
    pub scrollbar: bool,
}

fn default_scrollbar() -> bool {
    true
}

fn config_root() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join(".config"))
}

fn config_dir() -> Option<PathBuf> {
    config_root().map(|d| d.join("mdr"))
}

fn config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

pub fn load_config() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => {
            return Config {
                scrollbar: true,
                ..Default::default()
            };
        }
    };
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            return Config {
                scrollbar: true,
                ..Default::default()
            };
        }
    };

    toml::from_str(&contents).unwrap_or(Config {
        scrollbar: true,
        ..Default::default()
    })
}

/// Load the selected theme path and the explicit picker catalog.
pub fn load_theme_configs(config: &Config) -> BTreeMap<String, ThemeConfig> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut themes = BTreeMap::new();

    if let Some(catalog_path) = config.theme_catalog.as_deref() {
        for path in load_catalog_paths(&expand_home(&home, catalog_path), &home) {
            load_theme_config(&mut themes, &path);
        }
    }
    if let Some(theme_path) = config.theme.as_deref() {
        load_theme_config(&mut themes, &expand_home(&home, theme_path));
    }

    themes
}

pub fn configured_theme_name(config: &Config) -> Option<String> {
    let home = dirs::home_dir().unwrap_or_default();
    config
        .theme
        .as_deref()
        .map(|path| theme_name(&expand_home(&home, path)))
}

fn load_catalog_paths(catalog_path: &Path, home: &Path) -> Vec<PathBuf> {
    let Ok(contents) = fs::read_to_string(catalog_path) else {
        return Vec::new();
    };
    let Ok(catalog) = contents.parse::<toml::Value>() else {
        return Vec::new();
    };
    catalog
        .get("themes")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .map(|path| expand_home(home, path))
        .collect()
}

fn load_theme_config(themes: &mut BTreeMap<String, ThemeConfig>, path: &Path) {
    if let Ok(contents) = fs::read_to_string(path) {
        if let Ok(config) = toml::from_str::<ThemeConfig>(&contents) {
            themes.insert(theme_name(path), config);
        }
    }
}

fn expand_home(home: &Path, configured_path: &str) -> PathBuf {
    configured_path
        .strip_prefix("~/")
        .map(|rest| home.join(rest))
        .unwrap_or_else(|| PathBuf::from(configured_path))
}

fn theme_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("theme")
        .replace('-', " ")
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{load_theme_configs, Config};

    #[test]
    fn catalog_loads_only_explicit_theme_paths() {
        let root = test_root();
        let themes_dir = root.join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();
        std::fs::write(
            themes_dir.join("synthetic-theme.toml"),
            "[colors]\naccent = \"#112233\"\n[ui]\naccent = \"accent\"\n",
        )
        .unwrap();
        std::fs::write(
            themes_dir.join("unlisted.toml"),
            "[colors]\naccent = \"#abcdef\"\n[ui]\naccent = \"accent\"\n",
        )
        .unwrap();
        let catalog = root.join("catalog.toml");
        std::fs::write(
            &catalog,
            format!(
                "themes = [\"{}\"]\n",
                themes_dir.join("synthetic-theme.toml").display()
            ),
        )
        .unwrap();

        let config = Config {
            theme: Some(
                themes_dir
                    .join("synthetic-theme.toml")
                    .display()
                    .to_string(),
            ),
            theme_catalog: Some(catalog.display().to_string()),
            scrollbar: true,
        };
        let themes = load_theme_configs(&config);
        assert_eq!(themes.len(), 1);
        assert_eq!(themes["synthetic theme"].colors["accent"], "#112233");

        std::fs::remove_dir_all(root).unwrap();
    }

    fn test_root() -> std::path::PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "mdr-theme-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
}
