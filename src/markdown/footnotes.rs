use std::collections::HashMap;

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::theme::MarkdownTheme;

use super::blocks::push_rule_line;
use super::latex;
use super::links::{restore_linked_spans, LinkSpan, LinkedSpan};
use super::lists::{ItemState, ListKind};
use super::spans::InlineStyleState;
use super::tables::TableBuf;
use super::toc::TocEntry;
use super::width::display_width;
use super::wrapping::push_wrapped_prefixed_lines;
use super::{LastBlock, LineMapState};
use ratatui::style::Color;

pub(super) fn to_superscript(n: usize) -> String {
    n.to_string().chars().map(latex::to_superscript).collect()
}

pub(super) fn push_footnote_reference_span(
    spans: &mut Vec<LinkedSpan>,
    number: usize,
    theme: &MarkdownTheme,
) {
    spans.push(LinkedSpan::new(
        Span::styled(
            to_superscript(number),
            Style::default().fg(theme.footnote_ref),
        ),
        None,
    ));
}

pub(super) struct DefinitionSnapshot {
    pub(super) spans: Vec<LinkedSpan>,
    pub(super) lines: Vec<Line<'static>>,
    pub(super) link_spans: Vec<LinkSpan>,
    pub(super) list_stack: Vec<ListKind>,
    pub(super) item_stack: Vec<ItemState>,
    pub(super) blockquote_depth: usize,
    pub(super) in_code: bool,
    pub(super) code_lang: String,
    pub(super) code_buf: String,
    pub(super) inline: InlineStyleState,
    pub(super) blockquote_color: Option<Color>,
    pub(super) in_heading: Option<u8>,
    pub(super) table: Option<TableBuf>,
    pub(super) last_block: LastBlock,
    pub(super) state: LineMapState,
    pub(super) toc: Vec<TocEntry>,
}

impl DefinitionSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn take_from(
        spans: &mut Vec<LinkedSpan>,
        lines: &mut Vec<Line<'static>>,
        link_spans: &mut Vec<LinkSpan>,
        list_stack: &mut Vec<ListKind>,
        item_stack: &mut Vec<ItemState>,
        blockquote_depth: &mut usize,
        in_code: &mut bool,
        code_lang: &mut String,
        code_buf: &mut String,
        inline: &mut InlineStyleState,
        blockquote_color: &mut Option<Color>,
        in_heading: &mut Option<u8>,
        table: &mut Option<TableBuf>,
        last_block: &mut LastBlock,
        state: &mut LineMapState,
        toc: &mut Vec<TocEntry>,
    ) -> Self {
        Self {
            spans: std::mem::take(spans),
            lines: std::mem::take(lines),
            link_spans: std::mem::take(link_spans),
            list_stack: std::mem::take(list_stack),
            item_stack: std::mem::take(item_stack),
            blockquote_depth: std::mem::replace(blockquote_depth, 0),
            in_code: std::mem::replace(in_code, false),
            code_lang: std::mem::take(code_lang),
            code_buf: std::mem::take(code_buf),
            inline: std::mem::take(inline),
            blockquote_color: std::mem::take(blockquote_color),
            in_heading: std::mem::take(in_heading),
            table: std::mem::take(table),
            last_block: std::mem::replace(last_block, LastBlock::Other),
            state: std::mem::replace(state, LineMapState::new()),
            toc: std::mem::take(toc),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn restore_into(
        self,
        spans: &mut Vec<LinkedSpan>,
        lines: &mut Vec<Line<'static>>,
        link_spans: &mut Vec<LinkSpan>,
        list_stack: &mut Vec<ListKind>,
        item_stack: &mut Vec<ItemState>,
        blockquote_depth: &mut usize,
        in_code: &mut bool,
        code_lang: &mut String,
        code_buf: &mut String,
        inline: &mut InlineStyleState,
        blockquote_color: &mut Option<Color>,
        in_heading: &mut Option<u8>,
        table: &mut Option<TableBuf>,
        last_block: &mut LastBlock,
        state: &mut LineMapState,
        toc: &mut Vec<TocEntry>,
    ) {
        *spans = self.spans;
        *lines = self.lines;
        *link_spans = self.link_spans;
        *list_stack = self.list_stack;
        *item_stack = self.item_stack;
        *blockquote_depth = self.blockquote_depth;
        *in_code = self.in_code;
        *code_lang = self.code_lang;
        *code_buf = self.code_buf;
        *inline = self.inline;
        *blockquote_color = self.blockquote_color;
        *in_heading = self.in_heading;
        *table = self.table;
        *last_block = self.last_block;
        *state = self.state;
        *toc = self.toc;
    }
}

pub(super) struct ActiveDefinition {
    pub(super) label: String,
    pub(super) snapshot: DefinitionSnapshot,
}

#[derive(Default)]
pub(super) struct FootnotesBuf {
    refs_order: Vec<String>,
    refs_index: HashMap<String, usize>,
    defs_order: Vec<String>,
    definitions: HashMap<String, (Vec<Line<'static>>, Vec<LinkSpan>)>,
    def_source_line: HashMap<String, usize>,
    active: Option<ActiveDefinition>,
}

impl FootnotesBuf {
    pub(super) fn register_reference(&mut self, label: &str) -> usize {
        if let Some(&n) = self.refs_index.get(label) {
            return n;
        }
        let n = self.refs_index.len() + 1;
        self.refs_index.insert(label.to_string(), n);
        self.refs_order.push(label.to_string());
        n
    }

    pub(super) fn start_definition(
        &mut self,
        label: String,
        src_line: usize,
        snapshot: DefinitionSnapshot,
    ) {
        if !self.def_source_line.contains_key(&label) {
            self.defs_order.push(label.clone());
        }
        self.def_source_line.insert(label.clone(), src_line);
        self.active = Some(ActiveDefinition { label, snapshot });
    }

    pub(super) fn finish_definition(
        &mut self,
        captured_lines: Vec<Line<'static>>,
        captured_link_spans: Vec<LinkSpan>,
    ) -> DefinitionSnapshot {
        let ActiveDefinition { label, snapshot } = self
            .active
            .take()
            .expect("finish_definition without active definition");
        self.definitions
            .insert(label, (captured_lines, captured_link_spans));
        snapshot
    }

    pub(super) fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub(super) fn flush(
        &mut self,
        lines: &mut Vec<Line<'static>>,
        link_spans: &mut Vec<LinkSpan>,
        state: &mut LineMapState,
        theme: &MarkdownTheme,
        render_width: usize,
    ) {
        if self.definitions.is_empty() {
            return;
        }

        let mut ordered: Vec<(String, usize)> = Vec::new();
        for label in &self.refs_order {
            if let Some(&n) = self.refs_index.get(label) {
                ordered.push((label.clone(), n));
            }
        }
        let mut next_number = self.refs_index.len() + 1;
        for label in &self.defs_order {
            if self.refs_index.contains_key(label) {
                continue;
            }
            ordered.push((label.clone(), next_number));
            next_number += 1;
        }

        push_rule_line(lines, render_width, theme);
        state.mark_all_new(lines.len());
        let title = "Notes";
        lines.push(Line::from(Span::styled(
            title.to_string(),
            Style::default()
                .fg(theme.footnote_ref)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "─".repeat(display_width(title)),
            Style::default().fg(theme.heading_underline),
        )));
        state.mark_all_new(lines.len());

        for (label, number) in ordered {
            let Some((mut def_lines, def_link_spans)) = self.definitions.remove(&label) else {
                continue;
            };
            while def_lines.last().is_some_and(super::is_empty_line) {
                def_lines.pop();
            }
            if let Some(src) = self.def_source_line.get(&label) {
                state.current_src_line = *src;
            }
            let prefix_text = format!("{} ", to_superscript(number));
            let prefix_width = display_width(&prefix_text);
            let indent = " ".repeat(prefix_width);
            let mut first = true;
            for (line_idx, def_line) in def_lines.iter().enumerate() {
                let mut body = restore_linked_spans(line_idx, def_line, &def_link_spans);
                recolor_default_text(&mut body, theme.text, theme.footnote_text);
                if first {
                    first = false;
                    let first_prefix = vec![Span::styled(
                        prefix_text.clone(),
                        Style::default().fg(theme.footnote_ref),
                    )];
                    let continuation_prefix = vec![Span::raw(indent.clone())];
                    push_wrapped_prefixed_lines(
                        lines,
                        link_spans,
                        &mut body,
                        first_prefix,
                        continuation_prefix,
                        render_width,
                    );
                } else if body.is_empty() {
                    lines.push(Line::from(""));
                } else {
                    let first_prefix = vec![Span::raw(indent.clone())];
                    let continuation_prefix = vec![Span::raw(indent.clone())];
                    push_wrapped_prefixed_lines(
                        lines,
                        link_spans,
                        &mut body,
                        first_prefix,
                        continuation_prefix,
                        render_width,
                    );
                }
            }
            state.mark_all_new(lines.len());
        }

        lines.push(Line::from(""));
        state.mark_all_new(lines.len());
    }
}

fn recolor_default_text(spans: &mut [LinkedSpan], from: Color, to: Color) {
    for LinkedSpan { span, .. } in spans.iter_mut() {
        if span.style.fg == Some(from) {
            span.style.fg = Some(to);
        }
    }
}
