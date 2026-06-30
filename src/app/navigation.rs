use super::App;

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

    pub(crate) fn toggle_reverse_mode(&mut self) {
        self.reverse_mode = !self.reverse_mode;
    }

    pub(crate) fn scroll_down(&mut self, n: usize) {
        self.reset_numkey_state();
        self.scroll = (self.scroll + n).min(self.max_scroll());
        self.hovered_code_block = None;
    }

    pub(crate) fn scroll_up(&mut self, n: usize) {
        self.reset_numkey_state();
        self.scroll = self.scroll.saturating_sub(n);
        self.hovered_code_block = None;
    }

    pub(crate) fn scroll_top(&mut self) {
        self.reset_numkey_state();
        self.scroll = 0;
        self.hovered_code_block = None;
    }

    pub(crate) fn scroll_bottom(&mut self) {
        self.reset_numkey_state();
        self.scroll = self.max_scroll();
        self.hovered_code_block = None;
    }

    pub(crate) fn scroll_to(&mut self, position: usize) {
        self.reset_numkey_state();
        self.scroll = position.min(self.max_scroll());
        self.hovered_code_block = None;
    }

    pub(crate) fn toggle_toc(&mut self) {
        self.toc_visible = !self.toc_visible;
        if !self.toc_visible {
            self.hovered_toc_idx = None;
        }
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
        if let Some(&entry_idx) = self.toc_display_entries.get(display_idx) {
            if let Some(entry) = self.toc.get(entry_idx) {
                self.scroll_to(entry.line);
            }
        }
    }

    pub(crate) fn cycle_numkey(&mut self, key: u8) {
        let group = self.toc_group_for_numkey(key);
        if group.is_empty() {
            return;
        }

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
}
