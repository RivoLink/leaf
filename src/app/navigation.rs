use super::{App, TocScrollMode};

pub(super) enum CycleDirection {
    Forward,
    Backward,
}

pub(super) struct NumkeyCycleState {
    pub(super) key: u8,
    pub(super) position: usize,
}

impl App {
    pub(crate) fn max_scroll(&self) -> usize {
        if let Some(state) = self.diff() {
            if state.preview_visible && !state.files.is_empty() {
                let idx = state
                    .tree_active_idx
                    .min(state.files.len().saturating_sub(1));
                if let Some(pair) = state.preview_cache.get(&idx) {
                    let longest = pair.before_lines.len().max(pair.after_lines.len());

                    let avail = (self.content_area.height as usize).saturating_sub(2);
                    return longest.saturating_sub(avail);
                }
            }

            if matches!(state.layout, crate::diff::DiffLayout::Split) {
                return state
                    .split_lines
                    .len()
                    .saturating_sub(self.content_area.height as usize);
            }
        }
        self.total()
            .saturating_sub(self.content_area.height as usize)
    }

    pub(crate) fn visible_end(&self) -> usize {
        (self.scroll + self.content_area.height as usize).min(self.total())
    }

    pub(crate) fn scroll_percent(&self) -> u16 {
        let max = self.max_scroll();
        if max == 0 {
            return 100;
        }
        ((self.scroll * 100) / max).min(100) as u16
    }

    pub(super) fn reset_numkey_state(&mut self) {
        self.numkey_cycle = None;
        self.reverse_mode = false;
    }

    pub(super) fn sync_diff_file_from_scroll(&mut self) {
        if self.diff_state.is_none() {
            return;
        }
        let scroll = self.scroll;
        let resolved: Option<usize> = {
            let state = self.diff_state.as_ref().expect("checked");
            if state.preview_visible || state.files.is_empty() {
                return;
            }
            let owner: &[Option<usize>] = match state.layout {
                crate::diff::DiffLayout::Unified => &state.file_owner,
                crate::diff::DiffLayout::Split => &state.split_file_owner,
            };

            owner
                .get(scroll)
                .copied()
                .flatten()
                .or_else(|| owner[..scroll].iter().rev().flatten().next().copied())
                .or_else(|| owner.iter().skip(scroll).flatten().next().copied())
        };
        let state = self.diff_state.as_mut().expect("checked");
        state.nav_pinned_file_idx = None;
        if let Some(fi) = resolved {
            state.current_file_idx = fi.min(state.files.len() - 1);
        }
    }

    pub(crate) fn toggle_reverse_mode(&mut self) {
        self.reverse_mode = !self.reverse_mode;
    }

    pub(crate) fn scroll_down(&mut self, n: usize) {
        self.reset_numkey_state();
        self.reset_toc_scroll_mode();
        self.dismiss_diff_tree_hint_if_visible();
        self.scroll = (self.scroll + n).min(self.max_scroll());
        self.sync_diff_file_from_scroll();
    }

    pub(crate) fn scroll_up(&mut self, n: usize) {
        self.reset_numkey_state();
        self.reset_toc_scroll_mode();
        self.dismiss_diff_tree_hint_if_visible();
        self.scroll = self.scroll.saturating_sub(n);
        self.sync_diff_file_from_scroll();
    }

    pub(crate) fn scroll_top(&mut self) {
        self.reset_numkey_state();
        self.reset_toc_scroll_mode();
        self.dismiss_diff_tree_hint_if_visible();
        self.scroll = 0;
        self.sync_diff_file_from_scroll();
    }

    pub(crate) fn scroll_bottom(&mut self) {
        self.reset_numkey_state();
        self.reset_toc_scroll_mode();
        self.dismiss_diff_tree_hint_if_visible();
        self.scroll = self.max_scroll();
        self.sync_diff_file_from_scroll();
    }

    pub(crate) fn scroll_to(&mut self, position: usize) {
        self.reset_numkey_state();
        self.reset_toc_scroll_mode();
        self.dismiss_diff_tree_hint_if_visible();
        self.scroll = position.min(self.max_scroll());
        self.sync_diff_file_from_scroll();
    }

    pub(crate) fn toggle_toc(&mut self) {
        self.toc_visible = !self.toc_visible;
        if !self.toc_visible {
            self.hovered_toc_idx = None;
        }
        self.reset_toc_scroll_mode();
    }

    pub(crate) fn toggle_line_numbers(&mut self) {
        self.line_number_visible = !self.line_number_visible;
    }

    fn toc_group_for_numkey(&self, key: u8) -> Vec<usize> {
        let mut group = Vec::new();
        let mut top_level_index = 0u8;
        let mut collecting = false;

        for (idx, _entry, display_level) in self.visible_toc_entries() {
            if display_level == 1 {
                if collecting {
                    break;
                }
                top_level_index += 1;
                if top_level_index == key {
                    collecting = true;
                    group.push(idx);
                }
            } else if collecting {
                group.push(idx);
            }
        }
        group
    }

    pub(crate) fn scroll_to_toc_display_line(&mut self, display_idx: usize) {
        let Some(&entry_idx) = self.toc_display_entries.get(display_idx) else {
            return;
        };
        let Some(entry_line) = self.toc.get(entry_idx).map(|e| e.line) else {
            return;
        };
        let preserved_offset = self.current_toc_offset();
        self.reset_numkey_state();
        self.scroll = entry_line.min(self.max_scroll());
        self.toc_scroll_mode = TocScrollMode::Manual(preserved_offset);
        self.toc_active_pin = Some(entry_idx);
    }

    pub(crate) fn cycle_numkey(&mut self, key: u8) {
        let group = self.toc_group_for_numkey(key);
        if group.is_empty() {
            return;
        }

        self.reset_toc_scroll_mode();

        let direction = if self.reverse_mode {
            CycleDirection::Backward
        } else {
            CycleDirection::Forward
        };

        let position = match self.numkey_cycle.as_ref().filter(|s| s.key == key) {
            Some(state) => match direction {
                CycleDirection::Forward => (state.position + 1) % group.len(),
                CycleDirection::Backward => (state.position + group.len() - 1) % group.len(),
            },
            None => {
                self.reverse_mode = false;
                0
            }
        };

        self.numkey_cycle = Some(NumkeyCycleState { key, position });
        self.scroll = self.toc[group[position]].line.min(self.max_scroll());
    }

    pub(crate) fn toc_half_page_step(&self) -> usize {
        self.toc_list_area
            .map(|r| (r.height / 2) as usize)
            .unwrap_or(10)
            .max(1)
    }

    pub(crate) fn can_scroll_toc(&self) -> bool {
        self.is_toc_visible() && self.has_toc()
    }

    #[cfg(test)]
    pub(crate) fn toc_scroll_mode(&self) -> TocScrollMode {
        self.toc_scroll_mode
    }

    #[cfg(test)]
    pub(crate) fn is_toc_scroll_hint_dismissed(&self) -> bool {
        self.toc_scroll_hint_dismissed
    }

    pub(crate) fn toc_overflows(&self, list_height: u16) -> bool {
        self.toc_display_lines.len() > (list_height as usize).saturating_sub(1)
    }

    pub(crate) fn is_toc_scroll_hint_visible(&self) -> bool {
        if !self.is_toc_visible() || !self.has_toc() || self.toc_scroll_hint_dismissed {
            return false;
        }
        self.toc_list_area
            .is_some_and(|area| self.toc_overflows(area.height))
    }

    pub(super) fn max_toc_scroll_offset(&self, list_height: u16) -> usize {
        (self.toc_display_lines.len() + 1).saturating_sub(list_height as usize)
    }

    pub(crate) fn toc_scroll_offset(&self, list_height: u16) -> usize {
        let max_offset = self.max_toc_scroll_offset(list_height);
        self.toc_scroll_mode.offset().min(max_offset)
    }

    pub(crate) fn reset_toc_scroll_mode(&mut self) {
        self.toc_scroll_mode = TocScrollMode::Auto(self.toc_scroll_mode.offset());
        self.toc_active_pin = None;
    }

    pub(crate) fn clear_toc_scroll_state(&mut self) {
        self.toc_scroll_mode = TocScrollMode::Auto(0);
        self.toc_active_pin = None;
        self.toc_scroll_hint_dismissed = false;
    }

    pub(crate) fn scroll_toc_down(&mut self, n: usize) {
        let current = self.current_toc_offset();
        self.set_toc_manual_offset(current.saturating_add(n));
    }

    pub(crate) fn scroll_toc_up(&mut self, n: usize) {
        let current = self.current_toc_offset();
        self.set_toc_manual_offset(current.saturating_sub(n));
    }

    pub(crate) fn can_scroll_diff_tree(&self) -> bool {
        self.diff_state
            .as_ref()
            .is_some_and(|s| s.tree_visible && !s.files.is_empty() && !s.preview_visible)
    }

    fn diff_tree_view_rows(&self) -> usize {
        self.diff_tree_list_area.map_or(0, |r| r.height as usize)
    }

    fn max_diff_tree_scroll(&self) -> usize {
        let Some(state) = self.diff_state.as_ref() else {
            return 0;
        };
        state.files.len().saturating_sub(self.diff_tree_view_rows())
    }

    pub(crate) fn scroll_diff_tree_down(&mut self, n: usize) {
        self.dismiss_diff_tree_hint_if_visible();
        let max = self.max_diff_tree_scroll();
        if let Some(state) = self.diff_state.as_mut() {
            state.tree_scroll = (state.tree_scroll + n).min(max);

            state.tree_scroll_manual = true;
        }
    }

    pub(crate) fn scroll_diff_tree_up(&mut self, n: usize) {
        self.dismiss_diff_tree_hint_if_visible();
        if let Some(state) = self.diff_state.as_mut() {
            state.tree_scroll = state.tree_scroll.saturating_sub(n);
            state.tree_scroll_manual = true;
        }
    }

    pub(super) fn dismiss_diff_tree_hint_if_visible(&mut self) {
        if self.is_diff_tree_scroll_hint_visible() {
            self.diff_tree_scroll_hint_dismissed = true;
        }
    }

    pub(crate) fn is_diff_tree_scroll_hint_visible(&self) -> bool {
        if self.diff_tree_scroll_hint_dismissed {
            return false;
        }
        let open = self
            .diff_state
            .as_ref()
            .is_some_and(|s| s.tree_visible && !s.files.is_empty() && !s.preview_visible);
        if !open {
            return false;
        }
        self.diff_tree_overflows()
    }

    pub(crate) fn focus_next_diff_tree_file(&mut self) {
        self.step_diff_tree_cursor(1);
    }

    pub(crate) fn focus_prev_diff_tree_file(&mut self) {
        self.step_diff_tree_cursor(-1);
    }

    pub(crate) fn focus_next_diff_tree_file_by(&mut self, n: isize) {
        self.step_diff_tree_cursor(n);
    }

    pub(crate) fn focus_prev_diff_tree_file_by(&mut self, n: isize) {
        self.step_diff_tree_cursor(-n);
    }

    fn step_diff_tree_cursor(&mut self, delta: isize) {
        self.dismiss_diff_tree_hint_if_visible();
        let view = self.diff_tree_view_rows();
        let target_file = {
            let Some(state) = self.diff_state.as_mut() else {
                return;
            };
            if state.files.is_empty() {
                return;
            }
            let last = (state.files.len() - 1) as isize;
            let next = (state.tree_active_idx as isize + delta).clamp(0, last) as usize;
            state.tree_active_idx = next;

            state.tree_scroll_manual = false;
            if view > 0 && next >= state.tree_scroll + view {
                state.tree_scroll = next + 1 - view;
            } else if next < state.tree_scroll {
                state.tree_scroll = next;
            }
            next
        };
        self.warp_main_content_to_tree_file(target_file);
    }

    pub(crate) fn ensure_diff_tree_cursor_visible(&mut self) {
        let view = self.diff_tree_view_rows();
        if view == 0 {
            return;
        }

        if self
            .diff_state
            .as_ref()
            .is_some_and(|s| s.tree_scroll_manual)
        {
            return;
        }
        let Some(active) = self.diff_current_file_from_scroll() else {
            return;
        };
        if let Some(state) = self.diff_state.as_mut() {
            if active < state.tree_scroll {
                state.tree_scroll = active;
            } else if active >= state.tree_scroll + view {
                state.tree_scroll = active + 1 - view;
            }
        }
    }

    pub(crate) fn diff_tree_overflows(&self) -> bool {
        let view = self.diff_tree_view_rows();
        if view == 0 {
            return false;
        }
        self.diff_state
            .as_ref()
            .is_some_and(|s| s.tree_visible && s.files.len() > view)
    }

    fn warp_main_content_to_tree_file(&mut self, target_file: usize) {
        let target_line = self
            .diff_state
            .as_ref()
            .and_then(|s| s.first_scroll_target_of_file(target_file));
        if let Some(state) = self.diff_state.as_mut() {
            state.current_file_idx = target_file;
            state.nav_pinned_file_idx = Some(target_file);
            state.tree_scroll_manual = false;
        }
        if let Some(line) = target_line {
            self.scroll = line;
        }
        self.clamp_scroll();
    }

    pub(crate) fn focus_next_top_level_toc(&mut self) {
        self.cycle_visible_top_level(CycleDirection::Forward);
    }

    pub(crate) fn focus_prev_top_level_toc(&mut self) {
        self.cycle_visible_top_level(CycleDirection::Backward);
    }

    fn cycle_visible_top_level(&mut self, direction: CycleDirection) {
        let visible_tops = self.visible_top_level_display_indices();
        if visible_tops.is_empty() {
            return;
        }

        self.dismiss_toc_scroll_hint_if_visible();

        let active_position = self.toc_active_display_idx.and_then(|active_idx| {
            if !self.is_display_idx_in_toc_window(active_idx) {
                return None;
            }
            visible_tops
                .iter()
                .rposition(|(d_idx, _)| *d_idx <= active_idx)
        });

        let target_display_idx = match active_position {
            Some(position) => {
                let len = visible_tops.len();
                let new_position = match direction {
                    CycleDirection::Forward => (position + 1) % len,
                    CycleDirection::Backward => (position + len - 1) % len,
                };
                visible_tops[new_position].0
            }
            None => match direction {
                CycleDirection::Forward => visible_tops.first().unwrap().0,
                CycleDirection::Backward => visible_tops.last().unwrap().0,
            },
        };

        self.scroll_to_toc_display_line(target_display_idx);
    }

    fn visible_top_level_display_indices(&self) -> Vec<(usize, usize)> {
        let Some(area) = self.toc_list_area else {
            return Vec::new();
        };
        let offset = self.toc_scroll_offset(area.height);
        let list_height = area.height as usize;
        self.visible_toc_entries()
            .enumerate()
            .skip(offset)
            .take(list_height)
            .filter(|(_, (_, _, dl))| *dl == 1)
            .map(|(display_idx, (entry_idx, _, _))| (display_idx, entry_idx))
            .collect()
    }

    fn is_display_idx_in_toc_window(&self, display_idx: usize) -> bool {
        let Some(area) = self.toc_list_area else {
            return false;
        };
        let offset = self.toc_scroll_offset(area.height);
        (offset..offset + area.height as usize).contains(&display_idx)
    }

    fn current_toc_offset(&self) -> usize {
        let list_height = self.toc_list_area.map_or(0, |r| r.height);
        self.toc_scroll_offset(list_height)
    }

    fn set_toc_manual_offset(&mut self, offset: usize) {
        self.dismiss_toc_scroll_hint_if_visible();
        let list_height = self.toc_list_area.map_or(0, |r| r.height);
        let max_offset = self.max_toc_scroll_offset(list_height);
        self.toc_scroll_mode = TocScrollMode::Manual(offset.min(max_offset));
        self.hovered_toc_idx = None;
    }

    fn dismiss_toc_scroll_hint_if_visible(&mut self) {
        if self.is_toc_scroll_hint_visible() {
            self.toc_scroll_hint_dismissed = true;
        }
    }
}
