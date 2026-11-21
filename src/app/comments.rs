use super::{App, CommentFlash};

const EXCERPT_MAX_CHARS: usize = 80;
pub(crate) const COMMENT_EXPORT_HEADER: &str =
    "I reviewed this document and have the following comments. Please address them.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Comment {
    pub(crate) file: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text: String,
    pub(crate) excerpt: String,
}

impl Comment {
    pub(crate) fn anchor(&self) -> String {
        format!("{}:{}", self.file, line_range(self.start, self.end))
    }

    fn covers(&self, file: &str, source_line: usize) -> bool {
        self.file == file && (self.start..=self.end).contains(&source_line)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommentDraft {
    pub(crate) comment: Comment,
    editing: Option<usize>,
}

#[derive(Default)]
pub(crate) struct CommentState {
    // Rendered line index; Some means comment mode is active.
    cursor: Option<usize>,
    range_start: Option<usize>,
    items: Vec<Comment>,
    draft: Option<CommentDraft>,
    uncopied: bool,
    // Bumped on every change so the status-bar cache knows to rebuild.
    revision: u64,
}

fn line_range(start: usize, end: usize) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{start}-{end}")
    }
}

pub(crate) fn format_comments(items: &[Comment]) -> String {
    let mut out = format!("{COMMENT_EXPORT_HEADER}\n\n");
    for (idx, comment) in items.iter().enumerate() {
        out.push_str(&format!(
            "{}. `{}` - {}\n",
            idx + 1,
            comment.anchor(),
            comment.text
        ));
        if !comment.excerpt.is_empty() {
            out.push_str(&format!("   > {}\n", comment.excerpt));
        }
    }
    out
}

pub(crate) fn source_excerpt(source: &str, start: usize, end: usize) -> String {
    let Some(line) = source
        .lines()
        .skip(start.saturating_sub(1))
        .take(end.saturating_sub(start) + 1)
        .map(str::trim)
        .find(|line| !line.is_empty())
    else {
        return String::new();
    };
    if line.chars().count() <= EXCERPT_MAX_CHARS {
        return line.to_string();
    }
    let mut truncated: String = line.chars().take(EXCERPT_MAX_CHARS - 3).collect();
    truncated.push_str("...");
    truncated
}

impl App {
    fn touch_comments(&mut self) {
        self.comments.revision = self.comments.revision.wrapping_add(1);
    }

    pub(crate) fn comment_revision(&self) -> u64 {
        self.comments.revision
    }

    pub(crate) fn comments(&self) -> &[Comment] {
        &self.comments.items
    }

    pub(crate) fn comment_draft(&self) -> Option<&CommentDraft> {
        self.comments.draft.as_ref()
    }

    pub(crate) fn is_comment_input(&self) -> bool {
        self.comments.draft.is_some()
    }

    pub(crate) fn is_comment_mode(&self) -> bool {
        self.comments.cursor.is_some()
    }

    pub(crate) fn comment_file_label(&self) -> String {
        if self.filepath().is_some() {
            self.relative_path_string()
        } else {
            "<stdin>".to_string()
        }
    }

    fn last_content_line(&self) -> usize {
        self.lines
            .iter()
            .rposition(|line| line.spans.iter().any(|span| !span.content.is_empty()))
            .unwrap_or(0)
    }

    fn clamped_cursor(&self, last: usize) -> Option<usize> {
        let cursor = self.comments.cursor?.min(last);
        // Keep the cursor on screen when the view moves without it (mouse wheel, search).
        let top = self.scroll.min(last);
        let bottom = self.visible_end().saturating_sub(1).min(last);
        if self.content_area.height == 0 || top > bottom {
            return Some(cursor);
        }
        Some(cursor.clamp(top, bottom))
    }

    pub(crate) fn comment_cursor(&self) -> Option<usize> {
        self.clamped_cursor(self.last_content_line())
    }

    pub(crate) fn comment_selection(&self) -> Option<(usize, usize)> {
        let last = self.last_content_line();
        let cursor = self.clamped_cursor(last)?;
        let start = self
            .comments
            .range_start
            .map_or(cursor, |line| line.min(last));
        Some((cursor.min(start), cursor.max(start)))
    }

    pub(crate) fn has_comment_range(&self) -> bool {
        self.comments.range_start.is_some()
    }

    // The line map points every row of a fenced block at its opening fence,
    // so code rows are resolved through the block's own row-to-line table.
    fn comment_source_line(&self, line_idx: usize) -> usize {
        let code_line = self.code_blocks.iter().find_map(|block| {
            let first_row = block.rendered_start + 1;
            let row = block.code_rows.get(line_idx.checked_sub(first_row)?)?;
            Some((block.rendered_start, row.line))
        });
        match code_line {
            Some((_, line)) if self.file_mode => line + 1,
            Some((fence, line)) => self.source_line_at(fence) + 1 + line,
            None => self.source_line_at(line_idx),
        }
    }

    fn comment_source_selection(&self) -> Option<(usize, usize)> {
        let (lo, hi) = self.comment_selection()?;
        let (a, b) = (self.comment_source_line(lo), self.comment_source_line(hi));
        Some((a.min(b), a.max(b)))
    }

    pub(crate) fn comment_source_label(&self) -> Option<String> {
        let (start, end) = self.comment_source_selection()?;
        Some(format!("L{}", line_range(start, end)))
    }

    pub(crate) fn enter_comment_mode(&mut self) -> bool {
        if self.mode != super::AppMode::Document || !self.has_content() {
            return false;
        }
        self.comments.cursor = Some(self.scroll.min(self.last_content_line()));
        self.comments.range_start = None;
        self.touch_comments();
        true
    }

    pub(crate) fn exit_comment_mode(&mut self) -> bool {
        let was_active = self.comments.cursor.take().is_some();
        self.comments.range_start = None;
        if was_active {
            self.touch_comments();
        }
        was_active
    }

    pub(crate) fn clear_comment_range(&mut self) -> bool {
        let had_range = self.comments.range_start.take().is_some();
        if had_range {
            self.touch_comments();
        }
        had_range
    }

    pub(crate) fn toggle_comment_range(&mut self) {
        if self.comments.range_start.take().is_none() {
            self.comments.range_start = self.comment_cursor();
        }
        self.touch_comments();
    }

    pub(crate) fn move_comment_cursor_to(&mut self, line: usize) {
        let line = line.min(self.last_content_line());
        self.comments.cursor = Some(line);
        if line < self.scroll {
            self.scroll_to(line);
        } else if line >= self.visible_end() {
            let height = (self.content_area.height as usize).max(1);
            self.scroll_to((line + 1).saturating_sub(height));
        }
        self.touch_comments();
    }

    pub(crate) fn move_comment_cursor_by(&mut self, delta: isize) {
        let Some(cursor) = self.comment_cursor() else {
            return;
        };
        self.move_comment_cursor_to(cursor.saturating_add_signed(delta));
    }

    pub(crate) fn move_comment_cursor_page(&mut self, down: bool) {
        let amount = self.page_scroll_amount().max(1) as isize;
        self.move_comment_cursor_by(if down { amount } else { -amount });
    }

    fn comment_index_at(&self, file: &str, line_idx: usize) -> Option<usize> {
        let source_line = self.comment_source_line(line_idx);
        self.comments
            .items
            .iter()
            .position(|comment| comment.covers(file, source_line))
    }

    pub(crate) fn is_line_commented(&self, file: &str, line_idx: usize) -> bool {
        self.comment_index_at(file, line_idx).is_some()
    }

    fn comment_index_at_cursor(&self) -> Option<usize> {
        self.comment_index_at(&self.comment_file_label(), self.comment_cursor()?)
    }

    pub(crate) fn comment_at_cursor(&self) -> Option<&Comment> {
        self.comment_index_at_cursor()
            .map(|idx| &self.comments.items[idx])
    }

    fn excerpt_for(&self, start: usize, end: usize) -> String {
        // Code files are wrapped in a synthetic fence line that the line map skips.
        let offset = usize::from(self.file_mode);
        source_excerpt(&self.source, start + offset, end + offset)
    }

    pub(crate) fn begin_comment_input(&mut self) {
        let editing = if self.comments.range_start.is_none() {
            self.comment_index_at_cursor()
        } else {
            None
        };
        let comment = if let Some(idx) = editing {
            self.comments.items[idx].clone()
        } else {
            let Some((start, end)) = self.comment_source_selection() else {
                return;
            };
            Comment {
                file: self.comment_file_label(),
                start,
                end,
                text: String::new(),
                excerpt: self.excerpt_for(start, end),
            }
        };
        self.comments.draft = Some(CommentDraft { comment, editing });
        self.touch_comments();
    }

    pub(crate) fn push_comment_char(&mut self, c: char) {
        if let Some(draft) = self.comments.draft.as_mut() {
            draft.comment.text.push(c);
        }
    }

    pub(crate) fn pop_comment_char(&mut self) {
        if let Some(draft) = self.comments.draft.as_mut() {
            draft.comment.text.pop();
        }
    }

    pub(crate) fn clear_comment_draft_text(&mut self) {
        if let Some(draft) = self.comments.draft.as_mut() {
            draft.comment.text.clear();
        }
    }

    pub(crate) fn cancel_comment_input(&mut self) {
        self.comments.draft = None;
        self.touch_comments();
    }

    pub(crate) fn confirm_comment_input(&mut self) {
        let Some(CommentDraft {
            mut comment,
            editing,
        }) = self.comments.draft.take()
        else {
            return;
        };
        self.touch_comments();
        comment.text = comment.text.trim().to_string();
        if comment.text.is_empty() {
            if let Some(idx) = editing {
                self.comments.items.remove(idx);
                self.comments.uncopied = true;
                self.set_comment_flash(CommentFlash::Deleted);
            }
            return;
        }
        match editing {
            Some(idx) => self.comments.items[idx] = comment,
            None => self.comments.items.push(comment),
        }
        self.comments.items.sort_by(|a, b| {
            (a.file.as_str(), a.start, a.end).cmp(&(b.file.as_str(), b.start, b.end))
        });
        self.comments.range_start = None;
        self.comments.uncopied = true;
        self.set_comment_flash(CommentFlash::Saved);
    }

    pub(crate) fn delete_comment_at_cursor(&mut self) {
        if let Some(idx) = self.comment_index_at_cursor() {
            self.comments.items.remove(idx);
            self.comments.uncopied = true;
            self.touch_comments();
            self.set_comment_flash(CommentFlash::Deleted);
        }
    }

    pub(crate) fn clear_all_comments(&mut self) {
        let count = self.comments.items.len();
        if count == 0 {
            return;
        }
        if !self.is_comment_flash_active(CommentFlash::ConfirmClear(count)) {
            self.set_comment_flash(CommentFlash::ConfirmClear(count));
            return;
        }
        self.comments.items.clear();
        self.comments.uncopied = false;
        self.touch_comments();
        self.set_comment_flash(CommentFlash::Cleared);
    }

    pub(crate) fn disarm_comment_clear(&mut self) {
        self.clear_comment_flash_if(|flash| matches!(flash, CommentFlash::ConfirmClear(_)));
    }

    pub(crate) fn disarm_comment_quit(&mut self) {
        self.clear_comment_flash_if(|flash| matches!(flash, CommentFlash::ConfirmQuit(_)));
    }

    /// Returns true when quitting may proceed; with uncopied comments the
    /// first press only shows a confirmation that the second press accepts.
    pub(crate) fn confirm_quit_with_comments(&mut self) -> bool {
        let count = self.comments.items.len();
        if !self.comments.uncopied
            || count == 0
            || self.is_comment_flash_active(CommentFlash::ConfirmQuit(count))
        {
            return true;
        }
        self.set_comment_flash(CommentFlash::ConfirmQuit(count));
        false
    }

    pub(crate) fn jump_to_comment(&mut self, forward: bool) {
        let Some(cursor) = self.comment_cursor() else {
            return;
        };
        let file = self.comment_file_label();
        let current = self.comment_source_line(cursor);
        let starts = self
            .comments
            .items
            .iter()
            .filter(|comment| comment.file == file)
            .map(|comment| comment.start);
        let target = if forward {
            starts.filter(|&start| start > current).min()
        } else {
            starts.filter(|&start| start < current).max()
        };
        let Some(target) = target else {
            return;
        };
        let line =
            (0..=self.last_content_line()).find(|&idx| self.comment_source_line(idx) >= target);
        if let Some(line) = line {
            self.move_comment_cursor_to(line);
        }
    }

    pub(crate) fn copy_comments(&mut self) {
        let count = self.comments.items.len();
        if count == 0 {
            self.set_comment_flash(CommentFlash::NothingToCopy);
            return;
        }
        if crate::clipboard::copy_to_clipboard(&format_comments(&self.comments.items)) {
            self.comments.uncopied = false;
            self.set_comment_flash(CommentFlash::Copied(count));
        } else {
            self.set_comment_flash(CommentFlash::CopyFailed);
        }
    }
}
