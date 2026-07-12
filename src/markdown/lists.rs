use std::borrow::Cow;

use crate::theme::MarkdownTheme;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

use super::blocks::{block_prefix, trim_paragraph_gap_before_block};
use super::width::display_width;
use super::wrapping::push_wrapped_prefixed_lines;
use super::LastBlock;

pub(crate) const TASK_CHECKED: &str = "☑ ";
pub(crate) const TASK_CHECKED_ALT: &str = "☒ ";
pub(crate) const TASK_UNCHECKED: &str = "☐ ";

#[derive(Clone, Copy)]
pub(super) enum ListKind {
    Unordered,
    Ordered(u64),
}

pub(super) struct ItemState {
    pub(super) marker_emitted: bool,
    pub(super) continuation_indent: usize,
    pub(super) checkbox: Option<bool>,
    pub(super) opened_at_bq_depth: usize,
}

pub(super) fn list_item_prefix(
    blockquote_depth: usize,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
    theme: &MarkdownTheme,
    marker_color: Option<ratatui::style::Color>,
) -> Vec<Span<'static>> {
    let in_bq = blockquote_depth > 0;
    let Some(item) = item_stack.last_mut() else {
        return block_prefix(blockquote_depth, theme, marker_color);
    };

    let bq_is_outer = in_bq && blockquote_depth == item.opened_at_bq_depth;

    let mut prefix = Vec::new();

    if bq_is_outer {
        prefix.extend(block_prefix(blockquote_depth, theme, marker_color));
    }

    if item.marker_emitted {
        prefix.push(Span::raw(" ".repeat(item.continuation_indent)));
    } else {
        let depth = list_stack.len();
        prefix.push(Span::raw("  ".repeat(depth.saturating_sub(1))));

        let marker: Cow<'static, str> = match item.checkbox {
            // Avoids U+2611 emoji rendering on Windows.
            Some(true) if cfg!(target_os = "windows") => TASK_CHECKED_ALT.into(),
            Some(true) => TASK_CHECKED.into(),
            Some(false) => TASK_UNCHECKED.into(),
            None => match list_stack.last().copied().unwrap_or(ListKind::Unordered) {
                ListKind::Unordered => match depth {
                    1 => "• ".into(),
                    2 => "◦ ".into(),
                    _ => "▸ ".into(),
                },
                ListKind::Ordered(n) => format!("{n}. ").into(),
            },
        };
        item.continuation_indent = 2 * depth.saturating_sub(1) + display_width(&marker);
        item.marker_emitted = true;

        let marker_style = match item.checkbox {
            Some(true) => Style::default().fg(theme.task_checked),
            Some(false) => Style::default().fg(theme.task_unchecked),
            None => match list_stack.last().copied().unwrap_or(ListKind::Unordered) {
                ListKind::Unordered => match depth {
                    1 => Style::default().fg(theme.list_level_1),
                    2 => Style::default().fg(theme.list_level_2),
                    _ => Style::default().fg(theme.list_level_3),
                },
                ListKind::Ordered(_) => Style::default().fg(theme.ordered_list),
            },
        };
        prefix.push(Span::styled(marker, marker_style));
    }

    if in_bq && !bq_is_outer {
        prefix.extend(block_prefix(blockquote_depth, theme, marker_color));
    }

    prefix
}

#[allow(clippy::too_many_arguments)]
pub(super) fn flush_list_item_spans(
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
    blockquote_depth: usize,
    render_width: usize,
    theme: &MarkdownTheme,
    marker_color: Option<ratatui::style::Color>,
) {
    if spans.is_empty() {
        return;
    }

    let first_prefix = list_item_prefix(
        blockquote_depth,
        list_stack,
        item_stack,
        theme,
        marker_color,
    );
    let continuation_prefix = list_item_prefix(
        blockquote_depth,
        list_stack,
        item_stack,
        theme,
        marker_color,
    );
    push_wrapped_prefixed_lines(
        lines,
        spans,
        first_prefix,
        continuation_prefix,
        render_width,
    );
}

pub(super) fn start_list(
    lines: &mut Vec<Line<'static>>,
    last_block: LastBlock,
    list_stack: &mut Vec<ListKind>,
    start: Option<u64>,
) {
    trim_paragraph_gap_before_block(lines, last_block);
    list_stack.push(match start {
        Some(n) => ListKind::Ordered(n),
        None => ListKind::Unordered,
    });
}

pub(super) fn end_list(lines: &mut Vec<Line<'static>>, list_stack: &mut Vec<ListKind>) {
    list_stack.pop();
    if list_stack.is_empty() {
        lines.push(Line::from(""));
    }
}

pub(super) fn start_item(item_stack: &mut Vec<ItemState>, blockquote_depth: usize) {
    item_stack.push(ItemState {
        marker_emitted: false,
        continuation_indent: 0,
        checkbox: None,
        opened_at_bq_depth: blockquote_depth,
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn end_item(
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    list_stack: &mut [ListKind],
    item_stack: &mut Vec<ItemState>,
    blockquote_depth: usize,
    render_width: usize,
    theme: &MarkdownTheme,
    marker_color: Option<ratatui::style::Color>,
) {
    flush_list_item_spans(
        lines,
        spans,
        list_stack,
        item_stack,
        blockquote_depth,
        render_width,
        theme,
        marker_color,
    );
    item_stack.pop();
    if let Some(ListKind::Ordered(next)) = list_stack.last_mut() {
        *next += 1;
    }
}
