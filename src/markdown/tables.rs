use crate::theme::app_theme;
use pulldown_cmark::{Alignment, Event as MdEvent, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::latex;
use super::links::{emit_linked_line, register_link, LinkId, LinkSpan, LinkedSpan};
use super::spans::{normalize_html_tag, HtmlBufferKind, HtmlTagName, HtmlTagOutcome};
use super::table_layout::{
    align_cell, cap_table_widths, fit_table_widths, fragments_display_width, min_table_cell_width,
    wrap_table_cell,
};
use super::width::display_width;

#[derive(Clone, Copy, Default)]
pub(super) struct CellInlineStyle {
    pub(super) bold: u8,
    pub(super) italic: u8,
    pub(super) strikethrough: u8,
    pub(super) underline: u8,
    pub(super) link_id: Option<LinkId>,
}

impl CellInlineStyle {
    pub(super) fn modifiers(&self) -> Modifier {
        let mut m = Modifier::empty();
        if self.bold > 0 {
            m |= Modifier::BOLD;
        }
        if self.italic > 0 {
            m |= Modifier::ITALIC;
        }
        if self.strikethrough > 0 {
            m |= Modifier::CROSSED_OUT;
        }
        if self.underline > 0 {
            m |= Modifier::UNDERLINED;
        }
        m
    }
}

#[derive(Clone)]
pub(super) enum CellFragment {
    Text(String, CellInlineStyle, bool, Option<LinkId>),
    Code(String, bool, Option<LinkId>),
    InlineMath(String, bool, Option<LinkId>),
    LinkMarker(CellInlineStyle),
    Mark(String, bool, Option<LinkId>),
    HardBreak,
}

impl CellFragment {
    pub(super) fn rendered_text(&self) -> String {
        match self {
            CellFragment::Text(t, _, _, _)
            | CellFragment::Code(t, _, _)
            | CellFragment::Mark(t, _, _) => t.clone(),
            CellFragment::InlineMath(t, _, _) => latex::to_unicode(t),
            CellFragment::LinkMarker(_) => super::with_link_marker(|m| m.to_string()),
            CellFragment::HardBreak => String::new(),
        }
    }

    pub(super) fn display_width(&self) -> usize {
        let w = display_width(&self.rendered_text());
        match self {
            CellFragment::Text(_, _, _, _) | CellFragment::LinkMarker(_) => w,
            CellFragment::HardBreak => 0,
            _ => w + 2,
        }
    }

    pub(super) fn link_id(&self) -> Option<LinkId> {
        match self {
            CellFragment::Text(_, _, _, id)
            | CellFragment::Code(_, _, id)
            | CellFragment::InlineMath(_, _, id)
            | CellFragment::Mark(_, _, id) => *id,
            CellFragment::LinkMarker(style) => style.link_id,
            CellFragment::HardBreak => None,
        }
    }

    pub(super) fn is_text(&self) -> bool {
        matches!(self, CellFragment::Text(_, _, _, _))
    }
}

pub(super) struct TableBuf {
    pub(super) alignments: Vec<Alignment>,
    rows: Vec<Vec<Vec<CellFragment>>>,
    header_count: usize,
    current_row: Vec<Vec<CellFragment>>,
    current_cell: Vec<CellFragment>,
    pub(super) in_header: bool,
    inline_style: CellInlineStyle,
    html_style_buffer: Option<(HtmlBufferKind, Option<LinkId>, String)>,
    key_column: Option<usize>,
    fill_width: bool,
}

struct TableBorder<'a> {
    left: &'a str,
    fill: &'a str,
    cross: &'a str,
    right: &'a str,
}

pub(super) fn handle_table_event(
    table: &mut Option<TableBuf>,
    ev: &MdEvent<'_>,
    lines: &mut Vec<Line<'static>>,
    render_width: usize,
    link_urls: &mut Vec<String>,
    link_spans: &mut Vec<LinkSpan>,
) -> bool {
    let Some(tb) = table.as_mut() else {
        return false;
    };

    match ev {
        MdEvent::Text(t) => {
            tb.push_text(t.as_ref());
            true
        }
        MdEvent::Code(t) => {
            tb.push_code(t.as_ref());
            true
        }
        MdEvent::InlineMath(t) => {
            tb.push_inline_math(t.as_ref());
            true
        }
        MdEvent::Start(Tag::TableCell) => true,
        MdEvent::End(TagEnd::TableCell) => {
            tb.end_cell();
            true
        }
        MdEvent::Start(Tag::TableRow) => true,
        MdEvent::End(TagEnd::TableRow) => {
            tb.end_row();
            true
        }
        MdEvent::Start(Tag::TableHead) => {
            tb.in_header = true;
            true
        }
        MdEvent::End(TagEnd::TableHead) => {
            tb.end_header();
            true
        }
        MdEvent::Start(Tag::Strong) => {
            tb.inline_style.bold = tb.inline_style.bold.saturating_add(1);
            if tb.inline_style.link_id.is_some() {
                tb.update_link_marker_modifier(|s| s.bold = s.bold.saturating_add(1));
            }
            true
        }
        MdEvent::End(TagEnd::Strong) => {
            tb.inline_style.bold = tb.inline_style.bold.saturating_sub(1);
            true
        }
        MdEvent::Start(Tag::Emphasis) => {
            tb.inline_style.italic = tb.inline_style.italic.saturating_add(1);
            if tb.inline_style.link_id.is_some() {
                tb.update_link_marker_modifier(|s| s.italic = s.italic.saturating_add(1));
            }
            true
        }
        MdEvent::End(TagEnd::Emphasis) => {
            tb.inline_style.italic = tb.inline_style.italic.saturating_sub(1);
            true
        }
        MdEvent::Start(Tag::Strikethrough) => {
            tb.inline_style.strikethrough = tb.inline_style.strikethrough.saturating_add(1);
            if tb.inline_style.link_id.is_some() {
                tb.update_link_marker_modifier(|s| {
                    s.strikethrough = s.strikethrough.saturating_add(1)
                });
            }
            true
        }
        MdEvent::End(TagEnd::Strikethrough) => {
            tb.inline_style.strikethrough = tb.inline_style.strikethrough.saturating_sub(1);
            true
        }
        MdEvent::Start(Tag::Link { dest_url, .. }) => {
            tb.inline_style.link_id = Some(register_link(link_urls, dest_url.as_ref()));
            tb.push_link_marker();
            true
        }
        MdEvent::End(TagEnd::Link) => {
            tb.inline_style.link_id = None;
            true
        }
        MdEvent::InlineHtml(raw) => {
            match handle_html_tag_event_cell(raw.as_ref(), tb) {
                HtmlTagOutcome::Consumed => {}
                HtmlTagOutcome::OpenStyleBuffer(kind) => {
                    if tb.html_style_buffer.is_none() {
                        tb.html_style_buffer = Some((kind, tb.inline_style.link_id, String::new()));
                    }
                }
                HtmlTagOutcome::CloseStyleBuffer(kind) => {
                    if matches!(&tb.html_style_buffer, Some((k, _, _)) if *k == kind) {
                        tb.flush_html_style_buffer();
                    }
                }
                HtmlTagOutcome::HardBreak => {
                    tb.flush_html_style_buffer();
                    tb.push_hard_break();
                }
                HtmlTagOutcome::NotRecognized => {
                    tb.push_text(raw.as_ref());
                }
            }
            true
        }
        MdEvent::End(TagEnd::Table) => {
            tb.render(render_width, lines, link_spans);
            *table = None;
            true
        }
        _ => true,
    }
}

pub(super) fn start_table(table: &mut Option<TableBuf>, aligns: &[Alignment]) {
    *table = Some(TableBuf::new(aligns.to_vec()));
}

pub(super) fn handle_html_tag_event_cell(raw: &str, tb: &mut TableBuf) -> HtmlTagOutcome {
    let Some((tag, is_open)) = normalize_html_tag(raw) else {
        return HtmlTagOutcome::NotRecognized;
    };
    match (tag, is_open) {
        (HtmlTagName::Bold, true) => {
            tb.inline_style.bold = tb.inline_style.bold.saturating_add(1);
            if tb.inline_style.link_id.is_some() {
                tb.update_link_marker_modifier(|s| s.bold = s.bold.saturating_add(1));
            }
        }
        (HtmlTagName::Bold, false) => {
            tb.inline_style.bold = tb.inline_style.bold.saturating_sub(1);
        }
        (HtmlTagName::Italic, true) => {
            tb.inline_style.italic = tb.inline_style.italic.saturating_add(1);
            if tb.inline_style.link_id.is_some() {
                tb.update_link_marker_modifier(|s| s.italic = s.italic.saturating_add(1));
            }
        }
        (HtmlTagName::Italic, false) => {
            tb.inline_style.italic = tb.inline_style.italic.saturating_sub(1);
        }
        (HtmlTagName::Strike, true) => {
            tb.inline_style.strikethrough = tb.inline_style.strikethrough.saturating_add(1);
            if tb.inline_style.link_id.is_some() {
                tb.update_link_marker_modifier(|s| {
                    s.strikethrough = s.strikethrough.saturating_add(1)
                });
            }
        }
        (HtmlTagName::Strike, false) => {
            tb.inline_style.strikethrough = tb.inline_style.strikethrough.saturating_sub(1);
        }
        (HtmlTagName::Underline, true) => {
            tb.inline_style.underline = tb.inline_style.underline.saturating_add(1);
            if tb.inline_style.link_id.is_some() {
                tb.update_link_marker_modifier(|s| s.underline = s.underline.saturating_add(1));
            }
        }
        (HtmlTagName::Underline, false) => {
            tb.inline_style.underline = tb.inline_style.underline.saturating_sub(1);
        }
        (HtmlTagName::Mark, true) => return HtmlTagOutcome::OpenStyleBuffer(HtmlBufferKind::Mark),
        (HtmlTagName::Mark, false) => {
            return HtmlTagOutcome::CloseStyleBuffer(HtmlBufferKind::Mark)
        }
        (HtmlTagName::Code, true) => return HtmlTagOutcome::OpenStyleBuffer(HtmlBufferKind::Code),
        (HtmlTagName::Code, false) => {
            return HtmlTagOutcome::CloseStyleBuffer(HtmlBufferKind::Code)
        }
        (HtmlTagName::Br, _) => return HtmlTagOutcome::HardBreak,
    }
    HtmlTagOutcome::Consumed
}

impl TableBuf {
    fn new(alignments: Vec<Alignment>) -> Self {
        Self {
            alignments,
            rows: vec![],
            header_count: 0,
            current_row: vec![],
            current_cell: vec![],
            in_header: false,
            inline_style: CellInlineStyle::default(),
            html_style_buffer: None,
            key_column: None,
            fill_width: false,
        }
    }

    pub(crate) fn from_key_value_pairs(pairs: &[(String, String)], vertical: bool) -> Self {
        let style = CellInlineStyle::default();
        if vertical {
            let alignments = vec![Alignment::None; 2];
            let rows = pairs
                .iter()
                .map(|(k, v)| {
                    vec![
                        vec![CellFragment::Text(k.clone(), style, false, None)],
                        vec![CellFragment::Text(v.clone(), style, false, None)],
                    ]
                })
                .collect();
            Self {
                alignments,
                rows,
                header_count: 0,
                current_row: vec![],
                current_cell: vec![],
                in_header: false,
                inline_style: style,
                html_style_buffer: None,
                key_column: Some(0),
                fill_width: true,
            }
        } else {
            let alignments = vec![Alignment::None; pairs.len()];
            let header_row: Vec<Vec<CellFragment>> = pairs
                .iter()
                .map(|(k, _)| vec![CellFragment::Text(k.clone(), style, false, None)])
                .collect();
            let data_row: Vec<Vec<CellFragment>> = pairs
                .iter()
                .map(|(_, v)| vec![CellFragment::Text(v.clone(), style, false, None)])
                .collect();
            Self {
                alignments,
                rows: vec![header_row, data_row],
                header_count: 1,
                current_row: vec![],
                current_cell: vec![],
                in_header: false,
                inline_style: style,
                html_style_buffer: None,
                key_column: None,
                fill_width: true,
            }
        }
    }
    fn prev_ends_without_ws(&self) -> bool {
        match self.current_cell.last() {
            Some(CellFragment::Text(s, _, _, _)) => {
                !s.is_empty() && !s.ends_with(char::is_whitespace)
            }
            Some(_) => true,
            None => false,
        }
    }
    fn push_text(&mut self, t: &str) {
        use super::markers::{split_marker_segments, MarkerSegment, MARK_MARKER};

        if let Some((_, owner, buf)) = self.html_style_buffer.as_mut() {
            *owner = owner.or(self.inline_style.link_id);
            buf.push_str(t);
            return;
        }

        let starts_no_ws = !t.is_empty() && !t.starts_with(char::is_whitespace);
        let adjacent = starts_no_ws && self.prev_ends_without_ws();
        let style = self.inline_style;
        let segments = split_marker_segments(t, &[MARK_MARKER]);

        let mut first = true;
        for seg in &segments {
            let adj_flag = first && adjacent;
            first = false;
            match seg {
                MarkerSegment::Text(s) => {
                    self.current_cell.push(CellFragment::Text(
                        s.to_string(),
                        style,
                        adj_flag,
                        style.link_id,
                    ));
                }
                MarkerSegment::Mark(s) => {
                    self.current_cell.push(CellFragment::Mark(
                        s.to_string(),
                        adj_flag,
                        style.link_id,
                    ));
                }
            }
        }
    }
    fn push_link_marker(&mut self) {
        self.current_cell
            .push(CellFragment::LinkMarker(self.inline_style));
    }
    fn update_link_marker_modifier(&mut self, f: impl Fn(&mut CellInlineStyle)) {
        if let Some(CellFragment::LinkMarker(ref mut style)) = self
            .current_cell
            .iter_mut()
            .rev()
            .find(|frag| matches!(frag, CellFragment::LinkMarker(_)))
        {
            f(style);
        }
    }
    fn push_code(&mut self, t: &str) {
        if let Some((_, owner, buf)) = self.html_style_buffer.as_mut() {
            *owner = owner.or(self.inline_style.link_id);
            buf.push_str(t);
            return;
        }
        let adjacent = self.prev_ends_without_ws();
        self.current_cell.push(CellFragment::Code(
            t.to_string(),
            adjacent,
            self.inline_style.link_id,
        ));
    }
    fn push_inline_math(&mut self, t: &str) {
        if let Some((_, owner, buf)) = self.html_style_buffer.as_mut() {
            *owner = owner.or(self.inline_style.link_id);
            buf.push_str(t);
            return;
        }
        let adjacent = self.prev_ends_without_ws();
        self.current_cell.push(CellFragment::InlineMath(
            t.to_string(),
            adjacent,
            self.inline_style.link_id,
        ));
    }
    fn flush_html_style_buffer(&mut self) {
        if let Some((kind, link_id, text)) = self.html_style_buffer.take() {
            let adjacent = self.prev_ends_without_ws();
            match kind {
                HtmlBufferKind::Mark => {
                    self.current_cell
                        .push(CellFragment::Mark(text, adjacent, link_id));
                }
                HtmlBufferKind::Code => {
                    self.current_cell
                        .push(CellFragment::Code(text, adjacent, link_id));
                }
            }
        }
    }
    fn push_hard_break(&mut self) {
        self.current_cell.push(CellFragment::HardBreak);
    }
    fn end_cell(&mut self) {
        self.flush_html_style_buffer();
        let mut frags = std::mem::take(&mut self.current_cell);
        if let Some(CellFragment::Text(t, _, _, _)) = frags.first_mut() {
            *t = t.trim_start().to_string();
        }
        if let Some(CellFragment::Text(t, _, _, _)) = frags.last_mut() {
            *t = t.trim_end().to_string();
        }
        self.current_row.push(frags);
        self.inline_style = CellInlineStyle::default();
    }
    fn end_row(&mut self) {
        let row = std::mem::take(&mut self.current_row);
        if !row.is_empty() {
            self.rows.push(row);
        }
    }
    fn end_header(&mut self) {
        self.end_row();
        self.header_count = self.rows.len();
        self.in_header = false;
    }

    pub(crate) fn render(
        &self,
        render_width: usize,
        out: &mut Vec<Line<'static>>,
        ranges: &mut Vec<LinkSpan>,
    ) {
        let app_theme = app_theme();
        let theme = &app_theme.markdown;
        if self.rows.is_empty() {
            return;
        }
        let col_count = self.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if col_count == 0 {
            return;
        }

        let mut col_widths: Vec<usize> = vec![1; col_count];
        let mut min_widths: Vec<usize> = vec![4; col_count];
        for row in &self.rows {
            for (ci, cell) in row.iter().enumerate() {
                if ci < col_count {
                    col_widths[ci] = col_widths[ci].max(fragments_display_width(cell));
                    min_widths[ci] = min_widths[ci].max(min_table_cell_width(cell));
                }
            }
        }

        cap_table_widths(&mut col_widths, render_width);
        fit_table_widths(&mut col_widths, &min_widths, render_width);

        if self.fill_width {
            let border_width = 3 * col_count + 1;
            let available = render_width.saturating_sub(border_width);
            let current: usize = col_widths.iter().sum();
            if current < available {
                let extra = available - current;
                let base = extra / col_count;
                let remainder = extra % col_count;
                for (i, w) in col_widths.iter_mut().enumerate() {
                    *w += base + if i < remainder { 1 } else { 0 };
                }
            }
        }

        let border = Style::default().fg(theme.table_border);
        let sep = Style::default().fg(theme.table_separator);
        let header = Style::default()
            .fg(theme.table_header)
            .add_modifier(Modifier::BOLD);
        let cell = Style::default().fg(theme.table_cell);
        let ind = "";

        out.push(self.hline(
            ind,
            TableBorder {
                left: "┌",
                fill: "─",
                cross: "┬",
                right: "┐",
            },
            &col_widths,
            border,
        ));

        let empty_cell: Vec<CellFragment> = vec![];
        for (ri, row) in self.rows.iter().enumerate() {
            let is_hdr = ri < self.header_count;
            let wrapped_cells: Vec<Vec<Vec<CellFragment>>> = col_widths
                .iter()
                .copied()
                .enumerate()
                .take(col_count)
                .map(|(ci, width)| wrap_table_cell(row.get(ci).unwrap_or(&empty_cell), width))
                .collect();
            let row_height = wrapped_cells
                .iter()
                .map(|lines| lines.len())
                .max()
                .unwrap_or(1);

            for line_idx in 0..row_height {
                let mut spans = vec![
                    LinkedSpan::new(Span::raw(ind), None),
                    LinkedSpan::new(Span::styled("│", border), None),
                ];
                for (ci, width) in col_widths.iter().copied().enumerate().take(col_count) {
                    let frags = wrapped_cells[ci].get(line_idx).unwrap_or(&empty_cell);
                    let align = self.alignments.get(ci).copied().unwrap_or(Alignment::None);
                    let is_key_col = self.key_column == Some(ci);
                    let base_style = if is_hdr || is_key_col { header } else { cell };
                    let cell_spans =
                        align_cell(frags, width, align, base_style, is_hdr || is_key_col, theme);
                    spans.push(LinkedSpan::new(Span::raw(" "), None));
                    spans.extend(cell_spans);
                    spans.push(LinkedSpan::new(Span::raw(" "), None));
                    spans.push(LinkedSpan::new(Span::styled("│", border), None));
                }
                emit_linked_line(out, ranges, spans);
            }

            if is_hdr && ri == self.header_count - 1 {
                out.push(self.hline(
                    ind,
                    TableBorder {
                        left: "╞",
                        fill: "═",
                        cross: "╪",
                        right: "╡",
                    },
                    &col_widths,
                    sep,
                ));
            } else if !is_hdr && ri < self.rows.len() - 1 {
                out.push(self.hline(
                    ind,
                    TableBorder {
                        left: "├",
                        fill: "─",
                        cross: "┼",
                        right: "┤",
                    },
                    &col_widths,
                    border,
                ));
            }
        }

        out.push(self.hline(
            ind,
            TableBorder {
                left: "└",
                fill: "─",
                cross: "┴",
                right: "┘",
            },
            &col_widths,
            border,
        ));
        out.push(Line::from(""));
    }

    fn hline(
        &self,
        indent: &str,
        border: TableBorder<'_>,
        col_widths: &[usize],
        style: Style,
    ) -> Line<'static> {
        let mut spans = vec![
            Span::raw(indent.to_string()),
            Span::styled(border.left.to_string(), style),
        ];
        for (ci, &w) in col_widths.iter().enumerate() {
            spans.push(Span::styled(border.fill.repeat(w + 2), style));
            if ci < col_widths.len() - 1 {
                spans.push(Span::styled(border.cross.to_string(), style));
            }
        }
        spans.push(Span::styled(border.right.to_string(), style));
        Line::from(spans)
    }
}
