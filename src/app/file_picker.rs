use super::App;
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, TryRecvError},
    time::{Duration, Instant},
};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FilePickerEntry {
    label: String,
    path: PathBuf,
    label_lower: String,
    file_name: String,
    file_name_lower: String,
    file_name_offset: usize,
    path_depth: usize,
}

impl FilePickerEntry {
    pub(super) fn new(label: String, path: PathBuf) -> Self {
        let file_name = Self::file_name_component(&label).to_string();
        let file_name_offset = label
            .rfind(std::path::MAIN_SEPARATOR)
            .map(|idx| label[..idx + 1].chars().count())
            .unwrap_or(0);
        let path_depth = label.matches(std::path::MAIN_SEPARATOR).count();

        Self {
            label_lower: label.to_lowercase(),
            file_name_lower: file_name.to_lowercase(),
            label,
            path,
            file_name,
            file_name_offset,
            path_depth,
        }
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(super) fn label_lower(&self) -> &str {
        &self.label_lower
    }

    pub(super) fn file_name_lower(&self) -> &str {
        &self.file_name_lower
    }

    pub(super) fn file_name_offset(&self) -> usize {
        self.file_name_offset
    }

    pub(super) fn path_depth(&self) -> usize {
        self.path_depth
    }

    fn file_name_component(path: &str) -> &str {
        path.rsplit(std::path::MAIN_SEPARATOR)
            .next()
            .unwrap_or(path)
    }

    fn is_dir_like(&self) -> bool {
        self.label == ".." || self.label.ends_with('/')
    }
}

pub(crate) struct FilePickerState {
    pub(super) open: bool,
    pub(super) mode: FilePickerMode,
    pub(super) dir: PathBuf,
    pub(super) extras: Vec<String>,
    pub(super) entries: Vec<FilePickerEntry>,
    pub(super) filtered: Vec<usize>,
    pub(super) match_positions: Vec<Vec<usize>>,
    pub(super) index: usize,
    pub(super) query: String,
    pub(super) truncation: Option<PickerIndexTruncation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilePickerMode {
    Browser,
    Fuzzy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PendingPicker {
    None,
    Browser(PathBuf),
    Fuzzy(PathBuf),
    History,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickerIndexTruncation {
    Directory,
    File,
    Time,
}

pub(crate) struct PickerIndexResult {
    pub(crate) entries: Vec<FilePickerEntry>,
    pub(crate) truncated: Option<PickerIndexTruncation>,
}

pub(crate) enum PickerLoadState {
    Idle,
    Loading {
        mode: FilePickerMode,
        dir: PathBuf,
        started_at: Instant,
        receiver: Receiver<std::io::Result<PickerIndexResult>>,
        pending_result: Option<std::io::Result<PickerIndexResult>>,
    },
    LoadingHistory {
        started_at: Instant,
        receiver: Receiver<Vec<super::history::HistoryEntry>>,
        pending_result: Option<Vec<super::history::HistoryEntry>>,
    },
    Failed {
        mode: FilePickerMode,
        dir: PathBuf,
        message: String,
    },
}

impl App {
    pub(super) fn min_picker_loading_duration() -> Duration {
        Duration::from_millis(500)
    }

    fn selected_file_picker_entry(&self) -> Option<&FilePickerEntry> {
        let idx = *self.file_picker.filtered.get(self.file_picker.index)?;
        self.file_picker.entries.get(idx)
    }

    pub(crate) fn open_file_picker(&mut self, dir: PathBuf) -> bool {
        self.open_file_picker_with_mode(dir, FilePickerMode::Browser)
    }

    #[cfg(test)]
    pub(crate) fn open_fuzzy_file_picker(&mut self, dir: PathBuf) -> bool {
        self.open_file_picker_with_mode(dir, FilePickerMode::Fuzzy)
    }

    pub(crate) fn queue_file_picker(&mut self, dir: PathBuf) {
        self.pending_picker = PendingPicker::Browser(dir);
    }

    pub(crate) fn queue_fuzzy_file_picker(&mut self, dir: PathBuf) {
        self.pending_picker = PendingPicker::Fuzzy(dir);
    }

    pub(crate) fn has_pending_picker(&self) -> bool {
        !matches!(self.pending_picker, PendingPicker::None)
    }

    pub(crate) fn is_picker_loading(&self) -> bool {
        matches!(
            self.picker_load_state,
            PickerLoadState::Loading { .. } | PickerLoadState::LoadingHistory { .. }
        )
    }

    pub(crate) fn is_history_picker_loading(&self) -> bool {
        matches!(
            self.picker_load_state,
            PickerLoadState::LoadingHistory { .. }
        )
    }

    pub(crate) fn is_picker_load_failed(&self) -> bool {
        matches!(self.picker_load_state, PickerLoadState::Failed { .. })
    }

    pub(crate) fn pending_picker_mode(&self) -> Option<FilePickerMode> {
        match &self.picker_load_state {
            PickerLoadState::Loading { mode, .. } | PickerLoadState::Failed { mode, .. } => {
                Some(*mode)
            }
            PickerLoadState::LoadingHistory { .. } => None,
            PickerLoadState::Idle => match self.pending_picker {
                PendingPicker::Browser(..) => Some(FilePickerMode::Browser),
                PendingPicker::Fuzzy(..) => Some(FilePickerMode::Fuzzy),
                PendingPicker::History | PendingPicker::None => None,
            },
        }
    }

    pub(crate) fn pending_picker_dir(&self) -> Option<&std::path::Path> {
        match &self.picker_load_state {
            PickerLoadState::Loading { dir, .. } | PickerLoadState::Failed { dir, .. } => {
                Some(dir.as_path())
            }
            PickerLoadState::LoadingHistory { .. } => None,
            PickerLoadState::Idle => match &self.pending_picker {
                PendingPicker::Browser(dir) | PendingPicker::Fuzzy(dir) => Some(dir.as_path()),
                PendingPicker::History | PendingPicker::None => None,
            },
        }
    }

    pub(crate) fn picker_load_error(&self) -> Option<&str> {
        match &self.picker_load_state {
            PickerLoadState::Failed { message, .. } => Some(message.as_str()),
            PickerLoadState::Idle
            | PickerLoadState::Loading { .. }
            | PickerLoadState::LoadingHistory { .. } => None,
        }
    }

    pub(crate) fn poll_picker_loading(&mut self) -> bool {
        let state = std::mem::replace(&mut self.picker_load_state, PickerLoadState::Idle);
        match state {
            PickerLoadState::Loading {
                mode,
                dir,
                started_at,
                receiver,
                mut pending_result,
            } => {
                if pending_result.is_none() {
                    pending_result = match receiver.try_recv() {
                        Ok(result) => {
                            crate::runtime::debug_log(
                                self.debug_input,
                                &format!(
                                    "picker_loading worker_finished mode={mode:?} dir={}",
                                    dir.display()
                                ),
                            );
                            Some(result)
                        }
                        Err(TryRecvError::Empty) => None,
                        Err(TryRecvError::Disconnected) => Some(Err(std::io::Error::other(
                            "Picker loading worker disconnected",
                        ))),
                    };
                }

                if started_at.elapsed() < Self::min_picker_loading_duration() {
                    self.picker_load_state = PickerLoadState::Loading {
                        mode,
                        dir,
                        started_at,
                        receiver,
                        pending_result,
                    };
                    return false;
                }

                match pending_result {
                    Some(Ok(result)) => {
                        crate::runtime::debug_log(
                            self.debug_input,
                            &format!(
                                "picker_loading install mode={mode:?} dir={} entries={}",
                                dir.display(),
                                result.entries.len()
                            ),
                        );
                        self.install_loaded_file_picker(dir, mode, result)
                    }
                    Some(Err(err)) => {
                        crate::runtime::debug_log(
                            self.debug_input,
                            &format!(
                                "picker_loading failed mode={mode:?} dir={} error={}",
                                dir.display(),
                                err
                            ),
                        );
                        self.picker_load_state = PickerLoadState::Failed {
                            mode,
                            dir,
                            message: err.to_string(),
                        };
                        true
                    }
                    None => {
                        self.picker_load_state = PickerLoadState::Loading {
                            mode,
                            dir,
                            started_at,
                            receiver,
                            pending_result: None,
                        };
                        false
                    }
                }
            }
            PickerLoadState::LoadingHistory {
                started_at,
                receiver,
                mut pending_result,
            } => {
                if pending_result.is_none() {
                    pending_result = match receiver.try_recv() {
                        Ok(entries) => Some(entries),
                        Err(TryRecvError::Empty) => None,
                        Err(TryRecvError::Disconnected) => Some(Vec::new()),
                    };
                }

                if started_at.elapsed() < Self::min_picker_loading_duration() {
                    self.picker_load_state = PickerLoadState::LoadingHistory {
                        started_at,
                        receiver,
                        pending_result,
                    };
                    return false;
                }

                match pending_result {
                    Some(entries) => {
                        self.install_loaded_history_picker(entries);
                        true
                    }
                    None => {
                        self.picker_load_state = PickerLoadState::LoadingHistory {
                            started_at,
                            receiver,
                            pending_result: None,
                        };
                        false
                    }
                }
            }
            PickerLoadState::Failed { .. } => {
                self.picker_load_state = state;
                false
            }
            PickerLoadState::Idle => {
                self.picker_load_state = PickerLoadState::Idle;
                false
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn age_picker_loading_by(&mut self, duration: Duration) {
        if let PickerLoadState::Loading {
            mode,
            dir,
            started_at,
            receiver,
            pending_result,
        } = std::mem::replace(&mut self.picker_load_state, PickerLoadState::Idle)
        {
            let adjusted = started_at.checked_sub(duration).unwrap_or(started_at);
            self.picker_load_state = PickerLoadState::Loading {
                mode,
                dir,
                started_at: adjusted,
                receiver,
                pending_result,
            };
        }
    }

    fn open_file_picker_with_mode(&mut self, dir: PathBuf, mode: FilePickerMode) -> bool {
        let result = match mode {
            FilePickerMode::Browser => {
                Self::build_file_picker_entries(&dir, &self.file_picker.extras).map(|entries| {
                    PickerIndexResult {
                        entries,
                        truncated: None,
                    }
                })
            }
            FilePickerMode::Fuzzy => {
                Self::build_fuzzy_file_picker_entries(&dir, &self.file_picker.extras)
            }
        };

        match result {
            Ok(result) => self.install_loaded_file_picker(dir, mode, result),
            Err(_) => false,
        }
    }

    pub(crate) fn is_fuzzy_file_picker(&self) -> bool {
        self.file_picker.mode == FilePickerMode::Fuzzy
    }

    pub(crate) fn is_browser_file_picker(&self) -> bool {
        self.file_picker.mode == FilePickerMode::Browser
    }

    pub(crate) fn is_file_picker_open(&self) -> bool {
        self.file_picker.open
    }

    pub(crate) fn close_file_picker(&mut self) {
        self.file_picker.open = false;
        self.file_picker.query.clear();
        self.file_picker.entries.clear();
        self.file_picker.filtered.clear();
        self.file_picker.match_positions.clear();
        self.file_picker.index = 0;
        self.file_picker.truncation = None;
        self.picker_width_floor_active = false;
    }

    pub(crate) fn cancel_picker_loading(&mut self) {
        self.picker_load_state = PickerLoadState::Idle;
        self.pending_picker = PendingPicker::None;
        self.picker_width_floor_active = false;
    }

    pub(crate) fn file_picker_dir(&self) -> &std::path::Path {
        &self.file_picker.dir
    }

    pub(crate) fn file_picker_entries(&self) -> &[FilePickerEntry] {
        &self.file_picker.entries
    }

    pub(crate) fn file_picker_filtered_indices(&self) -> &[usize] {
        &self.file_picker.filtered
    }

    pub(crate) fn file_picker_match_positions(&self, filtered_idx: usize) -> &[usize] {
        self.file_picker
            .match_positions
            .get(filtered_idx)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn file_picker_index(&self) -> usize {
        self.file_picker.index
    }

    pub(crate) fn file_picker_query(&self) -> &str {
        &self.file_picker.query
    }

    pub(crate) fn file_picker_truncation(&self) -> Option<PickerIndexTruncation> {
        self.file_picker.truncation
    }

    pub(crate) fn move_file_picker_up(&mut self) {
        let total = self.file_picker.filtered.len();
        if total == 0 {
            return;
        }
        if self.file_picker.index == 0 {
            self.file_picker.index = total - 1;
        } else {
            self.file_picker.index -= 1;
        }
    }

    pub(crate) fn move_file_picker_down(&mut self) {
        let total = self.file_picker.filtered.len();
        if total == 0 {
            return;
        }
        self.file_picker.index = (self.file_picker.index + 1) % total;
    }

    pub(crate) fn push_file_picker_query(&mut self, ch: char) {
        if self.is_browser_file_picker() {
            return;
        }
        self.file_picker.query.push(ch);
        self.refresh_file_picker_matches();
    }

    pub(crate) fn pop_file_picker_query(&mut self) {
        if self.is_browser_file_picker() {
            return;
        }
        self.file_picker.query.pop();
        self.refresh_file_picker_matches();
    }

    pub(crate) fn clear_file_picker_query(&mut self) {
        if self.is_browser_file_picker() {
            return;
        }
        self.file_picker.query.clear();
        self.refresh_file_picker_matches();
    }

    pub(crate) fn open_file_picker_parent(&mut self) -> bool {
        if self.is_fuzzy_file_picker() {
            return false;
        }
        let Some(parent) = self.file_picker.dir.parent() else {
            return false;
        };
        self.open_file_picker(parent.to_path_buf())
    }

    pub(crate) fn activate_file_picker_selection(
        &mut self,
        ss: &SyntaxSet,
        themes: &ThemeSet,
    ) -> bool {
        let Some(entry) = self.selected_file_picker_entry().cloned() else {
            return false;
        };
        if self.is_browser_file_picker() && entry.is_dir_like() {
            self.open_file_picker(entry.path)
        } else {
            self.load_path(entry.path, ss, themes)
        }
    }
}
