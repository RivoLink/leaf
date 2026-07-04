use super::App;
use crate::markdown::hash_str;

const GOTO_LINE_CONTEXT_OFFSET: usize = 5;

pub(crate) struct GotoLineState {
    pub(super) mode: bool,
    pub(super) draft: String,
    pub(super) target: Option<usize>,
    pub(super) error: bool,
    pub(super) draft_hash: u64,
}

impl App {
    pub(crate) fn is_goto_line_mode(&self) -> bool {
        self.goto_line.mode
    }

    pub(crate) fn goto_line_draft(&self) -> &str {
        &self.goto_line.draft
    }

    pub(crate) fn has_active_goto_line(&self) -> bool {
        self.goto_line.target.is_some() || self.goto_line.error
    }

    pub(crate) fn goto_line_error(&self) -> bool {
        self.goto_line.error
    }

    pub(crate) fn goto_line_target(&self) -> Option<usize> {
        self.goto_line.target
    }

    pub(crate) fn begin_goto_line(&mut self) {
        self.reset_numkey_state();
        self.clear_active_search();
        self.goto_line.mode = true;
        self.goto_line.draft.clear();
        self.goto_line.draft_hash = 0;
        self.goto_line.error = false;
        self.goto_line.target = None;
        self.line_number_visible = true;
    }

    pub(crate) fn push_goto_draft(&mut self, ch: char) {
        if ch.is_ascii_digit() {
            self.goto_line.draft.push(ch);
            self.goto_line.draft_hash = hash_str(&self.goto_line.draft);
        }
    }

    pub(crate) fn pop_goto_draft(&mut self) {
        self.goto_line.draft.pop();
        self.goto_line.draft_hash = hash_str(&self.goto_line.draft);
    }

    pub(crate) fn confirm_goto_line(&mut self) {
        self.goto_line.mode = false;
        if self.goto_line.draft.is_empty() {
            self.clear_active_goto_line();
            return;
        }
        let logical = match self.goto_line.draft.parse::<usize>() {
            Ok(n) if n >= 1 => n,
            _ => {
                self.goto_line.error = true;
                return;
            }
        };
        if let Some(render_index) = self.find_render_index_for_logical(logical) {
            self.goto_line.target = Some(render_index);
            self.goto_line.error = false;
            self.reset_toc_scroll_mode();
            let scroll_pos = render_index.saturating_sub(GOTO_LINE_CONTEXT_OFFSET);
            self.scroll = scroll_pos.min(self.max_scroll());
        } else {
            self.goto_line.error = true;
        }
    }

    pub(crate) fn clear_active_goto_line(&mut self) {
        self.goto_line.mode = false;
        self.goto_line.draft.clear();
        self.goto_line.target = None;
        self.goto_line.error = false;
        self.goto_line.draft_hash = 0;
    }

    fn find_render_index_for_logical(&self, logical: usize) -> Option<usize> {
        self.line_number_map.iter().position(|&n| n == logical)
    }
}
