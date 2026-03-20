use std::fs;
use std::path::PathBuf;

fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join(".config").join("mdr").join("config.toml"))
}

pub fn load_theme_name() -> Option<String> {
    let path = config_path()?;
    let contents = fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("theme") {
            let value = value.trim_start();
            if let Some(value) = value.strip_prefix('=') {
                let value = value.trim();
                let value = value.trim_matches('"');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

pub fn save_theme_name(name: &str) {
    if let Some(path) = config_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, format!("theme = \"{name}\"\n"));
    }
}
