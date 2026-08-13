use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    sync::{mpsc::Sender, Mutex, OnceLock},
    thread,
    time::Duration,
};

use crate::config::history_path;

pub(crate) const MSG_HISTORY_DISABLED: &str =
    "Set a positive file-history-length in config to enable history";
pub(crate) const MSG_NO_FILE_HISTORY: &str = "No file history available";
pub(crate) const MSG_HISTORY_WRITE_FAILED: &str = "Failed to save file history";
pub(crate) const MSG_LOADING_FILE_HISTORY: &str = "Loading file history...";
pub(crate) const MSG_FILE_NO_LONGER_AVAILABLE: &str = "File no longer available";

pub(crate) fn msg_history_capped(was: i32) -> String {
    format!(
        "Config file-history-length was capped at {} (was {was})",
        crate::config::FILE_HISTORY_LENGTH_MAX
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HistoryEntry {
    pub(crate) path: PathBuf,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct HistoryFile {
    #[serde(default)]
    entries: Vec<HistoryEntry>,
}

pub(crate) fn load_history() -> Vec<HistoryEntry> {
    let Some(path) = history_path() else {
        return Vec::new();
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    toml::from_str::<HistoryFile>(&content)
        .map(|f| f.entries)
        .unwrap_or_default()
}

static HISTORY_ERROR_SENDER: OnceLock<Mutex<Option<Sender<HistoryWriteError>>>> = OnceLock::new();

#[derive(Clone, Copy, Debug)]
pub(crate) struct HistoryWriteError;

pub(crate) fn set_history_error_sender(sender: Sender<HistoryWriteError>) {
    let cell = HISTORY_ERROR_SENDER.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = cell.lock() {
        *guard = Some(sender);
    }
}

fn notify_write_failure() {
    let Some(cell) = HISTORY_ERROR_SENDER.get() else {
        return;
    };
    let Ok(guard) = cell.lock() else {
        return;
    };
    if let Some(sender) = guard.as_ref() {
        let _ = sender.send(HistoryWriteError);
    }
}

pub(crate) fn record_open(path: PathBuf, capacity: usize) {
    if capacity == 0 {
        return;
    }
    thread::spawn(move || {
        let canonical = std::fs::canonicalize(&path).unwrap_or(path);
        if let Err(()) = mutate_history(|history| {
            history.entries.retain(|e| e.path != canonical);
            history.entries.insert(0, HistoryEntry { path: canonical });
            if history.entries.len() > capacity {
                history.entries.truncate(capacity);
            }
        }) {
            notify_write_failure();
        }
    });
}

pub(crate) fn remove_paths(paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    thread::spawn(move || {
        if let Err(()) = mutate_history(|history| {
            history.entries.retain(|e| !paths.contains(&e.path));
        }) {
            notify_write_failure();
        }
    });
}

fn mutate_history<F>(mutate: F) -> std::result::Result<(), ()>
where
    F: FnOnce(&mut HistoryFile),
{
    let history_path = history_path().ok_or(())?;

    if let Some(parent) = history_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| ())?;
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&history_path)
        .map_err(|_| ())?;

    let mut locked = false;
    for &delay_ms in &[0u64, 50, 100, 200] {
        if delay_ms > 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }
        if FileExt::try_lock_exclusive(&file).is_ok() {
            locked = true;
            break;
        }
    }
    if !locked {
        return Err(());
    }

    let result = apply_history_mutation(&file, &history_path, mutate);
    let _ = FileExt::unlock(&file);
    result
}

fn apply_history_mutation<F>(
    mut file: &File,
    history_path: &std::path::Path,
    mutate: F,
) -> std::result::Result<(), ()>
where
    F: FnOnce(&mut HistoryFile),
{
    let mut existing = String::new();
    let _ = file.read_to_string(&mut existing);
    let mut history = if existing.is_empty() {
        HistoryFile::default()
    } else {
        toml::from_str::<HistoryFile>(&existing).unwrap_or_default()
    };

    mutate(&mut history);

    let serialized = toml::to_string(&history).map_err(|_| ())?;

    let tmp_path = history_path.with_extension("toml.tmp");
    {
        let mut tmp = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)
            .map_err(|_| ())?;
        tmp.write_all(serialized.as_bytes()).map_err(|_| ())?;
        tmp.sync_all().map_err(|_| ())?;
    }
    std::fs::rename(&tmp_path, history_path).map_err(|_| ())?;
    Ok(())
}
