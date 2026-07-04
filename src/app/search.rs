use super::App;
use crate::markdown::hash_str;

pub(crate) struct SearchState {
    pub(super) mode: bool,
    pub(super) draft: String,
    pub(super) query: String,
    pub(super) matches: Vec<usize>,
    pub(super) idx: usize,
    pub(super) draft_hash: u64,
    pub(super) query_hash: u64,
}

impl App {
    pub(crate) fn active_highlight_line(&self) -> Option<usize> {
        if self.search.matches.is_empty() {
            None
        } else {
            Some(self.search.matches[self.search.idx])
        }
    }

    pub(crate) fn is_search_mode(&self) -> bool {
        self.search.mode
    }

    pub(crate) fn search_draft(&self) -> &str {
        &self.search.draft
    }

    pub(crate) fn search_query(&self) -> &str {
        &self.search.query
    }

    #[cfg(test)]
    pub(crate) fn set_search_query(&mut self, query: impl Into<String>) {
        self.search.query = query.into();
        self.search.query_hash = hash_str(&self.search.query);
    }

    pub(crate) fn search_match_count(&self) -> usize {
        self.search.matches.len()
    }

    pub(crate) fn search_index(&self) -> usize {
        self.search.idx
    }

    #[cfg(test)]
    pub(crate) fn search_matches(&self) -> &[usize] {
        &self.search.matches
    }

    #[cfg(test)]
    pub(crate) fn set_search_draft(&mut self, draft: impl Into<String>) {
        self.search.draft = draft.into();
        self.search.draft_hash = hash_str(&self.search.draft);
    }

    pub(crate) fn pop_search_draft(&mut self) {
        self.search.draft.pop();
        self.search.draft_hash = hash_str(&self.search.draft);
    }

    pub(crate) fn push_search_draft(&mut self, ch: char) {
        self.search.draft.push(ch);
        self.search.draft_hash = hash_str(&self.search.draft);
    }

    pub(crate) fn run_search(&mut self) {
        let q = self.search.query.to_lowercase();
        if q.is_empty() {
            return;
        }
        let search_matches = {
            self.plain_lines
                .iter()
                .enumerate()
                .filter(|(_, line)| line.contains(&q))
                .map(|(i, _)| i)
                .collect()
        };
        self.search.matches = search_matches;
        self.search.idx = 0;
        if let Some(&f) = self.search.matches.first() {
            self.reset_toc_scroll_mode();
            self.scroll = f.min(self.max_scroll());
        }
    }

    pub(crate) fn begin_search(&mut self) {
        self.reset_numkey_state();
        self.search.mode = true;
        self.search.draft = self.search.query.clone();
        self.search.draft_hash = self.search.query_hash;
        crate::runtime::debug_log(
            self.debug_input,
            &format!(
                "begin_search query={:?} draft={:?} matches={} idx={}",
                self.search.query,
                self.search.draft,
                self.search.matches.len(),
                self.search.idx
            ),
        );
    }

    pub(crate) fn reset_search_state(&mut self) {
        self.search.draft.clear();
        self.search.query.clear();
        self.search.matches.clear();
        self.search.idx = 0;
        self.search.draft_hash = 0;
        self.search.query_hash = 0;
    }

    pub(crate) fn cancel_search(&mut self) {
        self.search.mode = false;
        self.reset_search_state();
        crate::runtime::debug_log(self.debug_input, "cancel_search cleared query and matches");
    }

    pub(crate) fn confirm_search(&mut self) {
        self.search.mode = false;
        let draft = std::mem::take(&mut self.search.draft);
        self.search.query = draft;
        self.search.query_hash = self.search.draft_hash;
        self.search.draft_hash = 0;
        if self.search.query.is_empty() {
            self.reset_search_state();
            crate::runtime::debug_log(
                self.debug_input,
                "confirm_search empty query -> cleared matches",
            );
            return;
        }
        self.run_search();
        crate::runtime::debug_log(
            self.debug_input,
            &format!(
                "confirm_search query={:?} matches={} idx={} scroll={}",
                self.search.query,
                self.search.matches.len(),
                self.search.idx,
                self.scroll
            ),
        );
    }

    pub(crate) fn clear_active_search(&mut self) {
        self.search.mode = false;
        self.reset_search_state();
        crate::runtime::debug_log(
            self.debug_input,
            "clear_active_search cleared query and matches",
        );
    }

    pub(crate) fn has_active_search(&self) -> bool {
        !self.search.query.is_empty() || !self.search.matches.is_empty()
    }

    pub(crate) fn next_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }
        self.reset_numkey_state();
        self.reset_toc_scroll_mode();
        self.search.idx = (self.search.idx + 1) % self.search.matches.len();
        self.scroll = self.search.matches[self.search.idx].min(self.max_scroll());
    }

    pub(crate) fn prev_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }
        self.reset_numkey_state();
        self.reset_toc_scroll_mode();
        if self.search.idx == 0 {
            self.search.idx = self.search.matches.len() - 1;
        } else {
            self.search.idx -= 1;
        }
        self.scroll = self.search.matches[self.search.idx].min(self.max_scroll());
    }
}
