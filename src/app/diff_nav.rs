use super::App;

impl App {
    pub(crate) fn diff_current_file_from_scroll(&self) -> Option<usize> {
        let state = self.diff_state.as_ref()?;
        if state.files.is_empty() {
            return None;
        }

        if let Some(pinned) = state.nav_pinned_file_idx {
            return Some(pinned.min(state.files.len() - 1));
        }

        Some(state.current_file_idx.min(state.files.len() - 1))
    }

    pub(crate) fn diff_next_file(&mut self) -> bool {
        let Some(cur) = self.diff_current_file_from_scroll() else {
            return false;
        };
        let Some(state) = self.diff_state.as_ref() else {
            return false;
        };
        let total_files = state.files.len();
        if total_files == 0 {
            return false;
        }
        let next_file = (cur + 1).min(total_files - 1);
        if next_file == cur {
            return false;
        }
        let target_line = state.first_scroll_target_of_file(next_file);
        let s = self.diff_state.as_mut().expect("checked");
        s.current_file_idx = next_file;
        s.nav_pinned_file_idx = Some(next_file);
        s.tree_active_idx = next_file;

        s.tree_scroll_manual = false;
        if let Some(line) = target_line {
            self.scroll = line;
        }
        self.clamp_scroll();
        true
    }

    pub(crate) fn diff_prev_file(&mut self) -> bool {
        let Some(cur) = self.diff_current_file_from_scroll() else {
            return false;
        };
        let Some(state) = self.diff_state.as_ref() else {
            return false;
        };
        if cur == 0 {
            return false;
        }
        let prev_file = cur - 1;
        let target_line = state.first_scroll_target_of_file(prev_file);
        let s = self.diff_state.as_mut().expect("checked");
        s.current_file_idx = prev_file;
        s.nav_pinned_file_idx = Some(prev_file);
        s.tree_active_idx = prev_file;
        s.tree_scroll_manual = false;
        if let Some(line) = target_line {
            self.scroll = line;
        }
        self.clamp_scroll();
        true
    }

    pub(crate) fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        if self.scroll > max {
            self.scroll = max;
        }
    }
}
