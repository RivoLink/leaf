use super::history::{
    HistoryEntry, MSG_FILE_NO_LONGER_AVAILABLE, MSG_HISTORY_DISABLED, MSG_NO_FILE_HISTORY,
};
use super::App;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

#[derive(Default)]
pub(crate) struct HistoryPickerState {
    pub(crate) open: bool,
    pub(crate) entries: Vec<HistoryEntry>,
    pub(crate) filenames_lower: Vec<String>,
    pub(crate) filtered: Vec<usize>,
    pub(crate) match_positions: Vec<Vec<usize>>,
    pub(crate) query: String,
    pub(crate) index: usize,
    pub(crate) error: Option<String>,
    pub(crate) activation_error: Option<String>,
}

impl App {
    pub(crate) fn queue_history_picker(&mut self) {
        self.pending_picker = super::file_picker::PendingPicker::History;
    }

    pub(crate) fn is_history_picker_open(&self) -> bool {
        self.history_picker.open
    }

    pub(crate) fn is_any_picker_active(&self) -> bool {
        self.has_pending_picker()
            || self.is_picker_loading()
            || self.is_file_picker_open()
            || self.is_history_picker_open()
    }

    pub(crate) fn close_history_picker(&mut self) {
        self.history_picker = HistoryPickerState::default();
        if self.is_history_picker_loading() {
            self.cancel_picker_loading();
        }
        self.picker_width_floor_active = false;
    }

    pub(crate) fn install_loaded_history_picker(&mut self, mut entries: Vec<HistoryEntry>) {
        let capacity = self.history_capacity();
        entries.truncate(capacity);
        self.history_picker.filenames_lower = entries
            .iter()
            .map(|e| {
                e.path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default()
            })
            .collect();
        self.history_picker.entries = entries;
        self.history_picker.query.clear();
        self.history_picker.index = 0;
        self.history_picker.error = if capacity == 0 {
            Some(MSG_HISTORY_DISABLED.to_string())
        } else if self.history_picker.entries.is_empty() {
            Some(MSG_NO_FILE_HISTORY.to_string())
        } else {
            None
        };
        self.history_picker.open = true;
        self.refresh_history_picker_matches();
    }

    pub(crate) fn history_picker_entries(&self) -> &[HistoryEntry] {
        &self.history_picker.entries
    }

    pub(crate) fn history_picker_filtered_indices(&self) -> &[usize] {
        &self.history_picker.filtered
    }

    pub(crate) fn history_picker_match_positions(&self, filtered_idx: usize) -> &[usize] {
        self.history_picker
            .match_positions
            .get(filtered_idx)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn history_picker_index(&self) -> usize {
        self.history_picker.index
    }

    pub(crate) fn history_picker_query(&self) -> &str {
        &self.history_picker.query
    }

    pub(crate) fn history_picker_error(&self) -> Option<&str> {
        self.history_picker.error.as_deref()
    }

    pub(crate) fn history_picker_activation_error(&self) -> Option<&str> {
        self.history_picker.activation_error.as_deref()
    }

    pub(crate) fn move_history_picker_up(&mut self) {
        self.history_picker.activation_error = None;
        let total = self.history_picker.filtered.len();
        if total == 0 {
            return;
        }
        if self.history_picker.index == 0 {
            self.history_picker.index = total - 1;
        } else {
            self.history_picker.index -= 1;
        }
    }

    pub(crate) fn move_history_picker_down(&mut self) {
        self.history_picker.activation_error = None;
        let total = self.history_picker.filtered.len();
        if total == 0 {
            return;
        }
        self.history_picker.index = (self.history_picker.index + 1) % total;
    }

    pub(crate) fn push_history_picker_query(&mut self, ch: char) {
        self.history_picker.activation_error = None;
        self.history_picker.query.push(ch);
        self.refresh_history_picker_matches();
    }

    pub(crate) fn pop_history_picker_query(&mut self) {
        self.history_picker.activation_error = None;
        self.history_picker.query.pop();
        self.refresh_history_picker_matches();
    }

    pub(crate) fn clear_history_picker_query(&mut self) {
        self.history_picker.activation_error = None;
        self.history_picker.query.clear();
        self.refresh_history_picker_matches();
    }

    pub(crate) fn refresh_history_picker_matches(&mut self) {
        let effective_total = self.history_picker.entries.len();
        let query = self.history_picker.query.trim().to_lowercase();

        if query.is_empty() {
            self.history_picker.filtered = (0..effective_total).collect();
            self.history_picker.match_positions.clear();
        } else {
            let mut scored: Vec<(usize, _, Vec<usize>)> = (0..effective_total)
                .filter_map(|idx| {
                    let filename = &self.history_picker.filenames_lower[idx];
                    super::fuzzy::fuzzy_component_match(filename, &query)
                        .map(|(score, positions)| (idx, score, positions))
                })
                .collect();
            scored.sort_by(|(l_idx, l_score, _), (r_idx, r_score, _)| {
                l_score.cmp(r_score).then_with(|| l_idx.cmp(r_idx))
            });
            self.history_picker.filtered = scored.iter().map(|(idx, _, _)| *idx).collect();
            self.history_picker.match_positions = scored.into_iter().map(|(_, _, p)| p).collect();
        }

        if self.history_picker.filtered.is_empty()
            || self.history_picker.index >= self.history_picker.filtered.len()
        {
            self.history_picker.index = 0;
        }
    }

    fn history_capacity(&self) -> usize {
        self.file_history_length()
            .filter(|n| *n > 0)
            .map(|n| n as usize)
            .unwrap_or(0)
    }

    pub(crate) fn activate_history_picker_selection(
        &mut self,
        ss: &SyntaxSet,
        themes: &ThemeSet,
    ) -> bool {
        let Some(&idx) = self.history_picker.filtered.get(self.history_picker.index) else {
            return false;
        };
        let Some(entry) = self.history_picker.entries.get(idx).cloned() else {
            return false;
        };
        if self.load_path(entry.path.clone(), ss, themes) {
            self.history_pending_removals.retain(|p| p != &entry.path);
            super::history::remove_paths(std::mem::take(&mut self.history_pending_removals));
            self.close_history_picker();
            true
        } else {
            if !self.history_pending_removals.contains(&entry.path) {
                self.history_pending_removals.push(entry.path);
            }
            self.history_picker.activation_error = Some(MSG_FILE_NO_LONGER_AVAILABLE.to_string());
            false
        }
    }
}
