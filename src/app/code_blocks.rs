use super::{App, CodeBlockFlash};
use crate::{clipboard::copy_to_clipboard, markdown::CodeBlockInfo};

impl App {
    pub(crate) fn set_code_blocks(&mut self, code_blocks: Vec<CodeBlockInfo>) {
        self.code_blocks = code_blocks;
    }

    pub(crate) fn is_code_select_mode(&self) -> bool {
        self.code_select.is_some()
    }

    pub(crate) fn exit_code_select_mode(&mut self) -> bool {
        let was_active = self.code_select.is_some();
        self.code_select = None;
        was_active
    }

    fn visible_code_block_indices(&self) -> Vec<usize> {
        let top = self.scroll;
        let bottom = self.visible_end();
        self.code_blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.rendered_end >= top && b.rendered_start < bottom)
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) fn copy_first_visible_code_block(&mut self) {
        let indices = self.visible_code_block_indices();
        match indices.first() {
            Some(&idx) => self.copy_code_block_at(idx),
            None => self.set_code_block_flash(CodeBlockFlash::NoneVisible),
        }
    }

    pub(crate) fn enter_code_select_mode(&mut self) {
        let indices = self.visible_code_block_indices();
        match indices.first() {
            Some(&idx) => self.code_select = Some(idx),
            None => self.set_code_block_flash(CodeBlockFlash::NoneVisible),
        }
    }

    pub(crate) fn code_select_next(&mut self) {
        let indices = self.visible_code_block_indices();
        if indices.is_empty() {
            self.code_select = None;
            return;
        }
        let current = self.code_select.unwrap_or(usize::MAX);
        let next = indices
            .iter()
            .copied()
            .find(|&i| i > current)
            .unwrap_or(indices[0]);
        self.code_select = Some(next);
    }

    pub(crate) fn code_select_prev(&mut self) {
        let indices = self.visible_code_block_indices();
        if indices.is_empty() {
            self.code_select = None;
            return;
        }
        let current = self.code_select.unwrap_or(0);
        let prev = indices
            .iter()
            .rev()
            .copied()
            .find(|&i| i < current)
            .unwrap_or_else(|| *indices.last().unwrap());
        self.code_select = Some(prev);
    }

    pub(crate) fn copy_selected_code_block(&mut self) {
        if let Some(idx) = self.code_select {
            self.copy_code_block_at(idx);
        }
        self.code_select = None;
    }

    pub(crate) fn copy_code_block_at(&mut self, idx: usize) {
        let raw = match self.code_blocks.get(idx) {
            Some(b) => b.raw_content.clone(),
            None => {
                self.set_code_block_flash(CodeBlockFlash::CopyFailed);
                return;
            }
        };
        if copy_to_clipboard(&raw) {
            self.set_code_block_flash(CodeBlockFlash::Copied);
        } else {
            self.set_code_block_flash(CodeBlockFlash::CopyFailed);
        }
    }

    pub(crate) fn code_block_at(&self, line_idx: usize, inner_col: u16) -> Option<usize> {
        let col = inner_col as usize;
        self.code_blocks.iter().position(|b| {
            line_idx >= b.rendered_start
                && line_idx <= b.rendered_end
                && col >= b.rendered_offset
                && col < b.rendered_offset + b.rendered_width
        })
    }

    pub(crate) fn highlighted_code_block(&self) -> Option<usize> {
        self.code_select
    }
}
