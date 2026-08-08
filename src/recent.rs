use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_RECENTS: usize = 50;

/// Load recently opened markdown documents, most-recent first.
pub fn load() -> Vec<PathBuf> {
    let Some(path) = recent_path() else {
        return Vec::new();
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    contents
        .lines()
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Record a document as the most recently opened entry.
pub fn record(path: &Path) {
    let Some(store) = recent_path() else {
        return;
    };
    let entries = promote(load(), path, MAX_RECENTS);
    if let Some(parent) = store.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let contents = entries
        .iter()
        .map(|entry| entry.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = fs::write(store, format!("{contents}\n"));
}

fn promote(mut entries: Vec<PathBuf>, path: &Path, max: usize) -> Vec<PathBuf> {
    entries.retain(|existing| existing != path);
    entries.insert(0, path.to_path_buf());
    entries.truncate(max);
    entries
}

fn recent_path() -> Option<PathBuf> {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(base.join("mdr").join("recent"))
}

#[cfg(test)]
mod tests {
    use super::promote;
    use std::path::PathBuf;

    #[test]
    fn promote_moves_existing_entry_to_front() {
        let entries = vec![
            PathBuf::from("/synthetic/a.md"),
            PathBuf::from("/synthetic/b.md"),
        ];

        let promoted = promote(entries, &PathBuf::from("/synthetic/b.md"), 50);

        assert_eq!(promoted[0], PathBuf::from("/synthetic/b.md"));
        assert_eq!(promoted.len(), 2);
    }
}
