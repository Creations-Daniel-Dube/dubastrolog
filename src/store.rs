use std::path::{Path, PathBuf};

use crate::model::Journal;

pub fn data_dir() -> PathBuf {
    if cfg!(windows) {
        let base = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        base.join("dubastrolog")
    } else {
        let base = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".config"))
            })
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("dubastrolog")
    }
}

pub fn journal_path() -> PathBuf {
    data_dir().join("journal.json")
}

pub fn media_dir() -> PathBuf {
    data_dir().join("media")
}

pub fn attachment_path(stored_name: &str) -> PathBuf {
    media_dir().join(stored_name)
}

pub fn load_journal() -> (Journal, Option<String>) {
    let path = journal_path();
    if !path.exists() {
        return (Journal::default(), None);
    }
    match load_from(&path) {
        Ok(journal) => (journal, None),
        Err(err) => (
            Journal::default(),
            Some(format!(
                "Le journal n'a pas pu être lu ({err}). Les modifications ne seront pas enregistrées, pour ne pas écraser le fichier."
            )),
        ),
    }
}

pub fn load_from(path: &Path) -> Result<Journal, String> {
    let text = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&text).map_err(|err| err.to_string())
}

pub fn save_journal(journal: &Journal) -> Result<(), String> {
    save_to(journal, &journal_path())
}

pub fn save_to(journal: &Journal, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let text = serde_json::to_string_pretty(journal).map_err(|err| err.to_string())?;
    std::fs::write(path, text).map_err(|err| err.to_string())
}

pub fn remove_stored(stored_name: &str) {
    if !is_safe_stored_name(stored_name) {
        return;
    }
    let _ = std::fs::remove_file(media_dir().join(stored_name));
}

fn is_safe_stored_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\\')
        && Path::new(name).components().count() == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Session, Site};

    #[test]
    fn save_and_load_roundtrip() {
        let dir =
            std::env::temp_dir().join(format!("dubastrolog-store-{}", crate::model::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("journal.json");
        let journal = Journal {
            sites: vec![Site {
                id: "s".to_string(),
                name: "Cour".to_string(),
                ..Site::default()
            }],
            sessions: vec![Session {
                id: "n".to_string(),
                date: "2026-03-01".to_string(),
                ..Session::default()
            }],
            ..Journal::default()
        };
        save_to(&journal, &path).unwrap();
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded, journal);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_journal_path_uses_default() {
        let dir =
            std::env::temp_dir().join(format!("dubastrolog-store-{}", crate::model::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("journal.json");
        assert!(!path.exists());

        let (journal, notice) = if !path.exists() {
            (Journal::default(), None)
        } else {
            match load_from(&path) {
                Ok(journal) => (journal, None),
                Err(err) => (Journal::default(), Some(err)),
            }
        };

        assert!(notice.is_none());
        assert_eq!(journal, Journal::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
