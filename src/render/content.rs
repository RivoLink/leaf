use crate::{
    app::{App, COPY_HIGHLIGHT_DURATION_MS},
    markdown::{
        width::{rendered_cluster_width, rendered_span_width},
        LinkId, LinkSpan,
    },
    theme::app_theme,
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};
use std::{collections::HashMap, time::Duration};
use unicode_segmentation::UnicodeSegmentation;

use super::{CONTENT_HORIZONTAL_PADDING, SCROLLBAR_WIDTH};

pub(super) fn render_content_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let viewport_height = area.height as usize;
    let theme = app_theme();
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.ui.content_bg)),
        area,
    );
    let content_area = inner_content_area(area);
    let scroll = app.scroll();
    let active_highlight_line = app.active_highlight_line();
    if let Some(line_idx) = active_highlight_line {
        let _ = app.refresh_highlighted_line_cache(line_idx);
    }

    let visible_end = (scroll + viewport_height).min(app.total());
    let mut visible_lines = app.visible_lines(scroll, visible_end).to_vec();

    for &row in app.search_matches() {
        if Some(row) != active_highlight_line && (scroll..visible_end).contains(&row) {
            let line = &mut visible_lines[row - scroll];
            *line =
                crate::markdown::highlight_matches_only(line, &theme.markdown, app.search_query());
        }
    }

    if let Some(line_idx) = active_highlight_line {
        if (scroll..visible_end).contains(&line_idx) {
            if let Some((_, highlighted_line)) = app.highlighted_line_cache() {
                visible_lines[line_idx - scroll] = highlighted_line.clone();
            }
        }
    }

    let hovered_id = app
        .hovered_link
        .and_then(|(line, index)| app.link_span(line, index))
        .map(|link| link.link_id);
    if let Some(id) = hovered_id {
        style_link_occurrence(
            &mut visible_lines,
            &app.link_spans_by_line,
            scroll,
            id,
            Style::default().fg(theme.markdown.link_hover),
        );
    }

    if let Some(id) = app.selected_link() {
        style_link_occurrence(
            &mut visible_lines,
            &app.link_spans_by_line,
            scroll,
            id,
            Style::default()
                .fg(theme.markdown.link_text)
                .bg(theme.ui.content_bg)
                .add_modifier(Modifier::REVERSED),
        );
    }

    if let Some(block_idx) = app.highlighted_code_block() {
        if let Some(block) = app.code_blocks.get(block_idx) {
            apply_code_block_highlight(
                &mut visible_lines,
                block,
                scroll,
                visible_end,
                &theme.markdown,
            );
        }
    }

    if let Some((ranges, started)) = app.copy_highlight() {
        if started.elapsed() < Duration::from_millis(COPY_HIGHLIGHT_DURATION_MS) {
            let style = Style::default().bg(theme.markdown.search_highlight_bg);
            for range in ranges {
                if let Some(line) = range
                    .line_idx
                    .checked_sub(scroll)
                    .and_then(|idx| visible_lines.get_mut(idx))
                {
                    apply_column_range_style(line, range.start_col, range.end_col, style);
                }
            }
        }
    }

    let comment_marks = comment_marks(app, scroll, visible_end);
    let selection_bg = theme.ui.toc_active_bg;
    for (line, mark) in visible_lines.iter_mut().zip(&comment_marks) {
        if *mark == CommentMark::Selected {
            for span in &mut line.spans {
                span.style = span.style.bg(selection_bg);
            }
        }
    }

    if app.is_line_number_visible() {
        let digit_width = app.line_number_total().max(1).to_string().len();
        let gutter_style = Style::default().fg(theme.markdown.code_gutter);
        let goto_target = app.goto_line_target();
        let highlight_bg = theme.markdown.search_highlight_bg;
        for (i, line) in visible_lines.iter_mut().enumerate() {
            let idx = scroll + i;
            let logical = app.line_number_at(idx);
            let is_first =
                logical > 0 && (idx == 0 || app.line_number_at(idx.saturating_sub(1)) != logical);
            let is_goto_target = goto_target == Some(idx);
            let num_part = if is_first {
                format!("{:>w$}", logical, w = digit_width)
            } else {
                format!("{:>w$}", "", w = digit_width)
            };
            if is_goto_target {
                let hl = gutter_style.bg(highlight_bg);
                line.spans.insert(0, Span::styled("│ ", hl));
                line.spans
                    .insert(0, Span::styled(num_part, hl.fg(Color::White)));
                for span in &mut line.spans[2..] {
                    span.style = span.style.bg(highlight_bg);
                }
            } else {
                let gutter = format!("{num_part}│ ");
                line.spans.insert(0, Span::styled(gutter, gutter_style));
            }
        }
    }

    app.refresh_screen_geometry(&visible_lines, content_area.width, content_area.height);
    f.render_widget(
        Paragraph::new(visible_lines)
            .style(Style::default().bg(theme.ui.content_bg))
            .wrap(Wrap { trim: false }),
        content_area,
    );
    draw_comment_marks(f, app, &comment_marks, scroll, area.x, content_area.y);

    let (mouse_col, mouse_row) = app.mouse_position;
    let sb_x = area
        .x
        .saturating_add(area.width)
        .saturating_sub(SCROLLBAR_WIDTH);
    let on_sb_column = mouse_col >= sb_x
        && mouse_col < sb_x + SCROLLBAR_WIDTH
        && mouse_row >= area.y
        && mouse_row < area.y + area.height;

    let max_scroll = app.max_scroll();
    let track_len = area.height as usize;
    let mouse_on_thumb = on_sb_column && track_len > 0 && max_scroll > 0 && {
        let thumb_size = (track_len * track_len / max_scroll).max(1).min(track_len);
        let max_offset = track_len.saturating_sub(thumb_size);
        let thumb_offset = app.scroll() * max_offset / max_scroll;
        let thumb_top = area.y as usize + thumb_offset;
        let thumb_bottom = thumb_top + thumb_size;
        let row = mouse_row as usize;
        row >= thumb_top && row < thumb_bottom
    };

    let mut scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(Some("│"))
        .thumb_symbol("█");
    if mouse_on_thumb || app.scrollbar_dragging {
        scrollbar = scrollbar.thumb_style(Style::default().fg(theme.ui.scrollbar_hover));
    }

    let mut scrollbar_state = ScrollbarState::new(max_scroll).position(app.scroll());
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

pub(crate) fn inner_content_area(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(CONTENT_HORIZONTAL_PADDING),
        y: area.y,
        width: area
            .width
            .saturating_sub(CONTENT_HORIZONTAL_PADDING.saturating_mul(2))
            .saturating_sub(SCROLLBAR_WIDTH),
        height: area.height,
    }
}

fn apply_code_block_highlight(
    visible_lines: &mut [ratatui::text::Line<'static>],
    block: &crate::markdown::CodeBlockInfo,
    scroll: usize,
    visible_end: usize,
    theme: &crate::theme::MarkdownTheme,
) {
    let start = block.rendered_start.max(scroll);
    let end = block.rendered_end.min(visible_end.saturating_sub(1));
    if start > end {
        return;
    }
    let footer_line = block.rendered_end.saturating_sub(1);
    let focus = theme.link_hover;
    for line_idx in start..=end {
        let vis_idx = line_idx - scroll;
        let Some(line) = visible_lines.get_mut(vis_idx) else {
            continue;
        };
        let target = if line_idx == block.rendered_start || line_idx == footer_line {
            theme.code_frame
        } else {
            theme.code_gutter
        };
        for span in &mut line.spans {
            if span.style.fg == Some(target) {
                span.style = span.style.fg(focus);
            }
        }
    }
}

fn apply_column_range_style(
    line: &mut ratatui::text::Line<'static>,
    start_col: usize,
    end_col: usize,
    style: Style,
) {
    let mut spans = Vec::with_capacity(line.spans.len() + 2);
    let mut col = 0usize;
    for span in line.spans.drain(..) {
        let span_start = col;
        col += rendered_span_width(&span);
        if col <= start_col || span_start >= end_col {
            spans.push(span);
            continue;
        }
        if span_start >= start_col && col <= end_col {
            spans.push(span.patch_style(style));
            continue;
        }
        let mut parts = [String::new(), String::new(), String::new()];
        let mut cluster_col = span_start;
        for cluster in span.content.graphemes(true) {
            let part = if cluster_col < start_col {
                0
            } else if cluster_col < end_col {
                1
            } else {
                2
            };
            parts[part].push_str(cluster);
            cluster_col += rendered_cluster_width(cluster);
        }
        let [before, inside, after] = parts;
        for (text, part_style) in [
            (before, span.style),
            (inside, span.style.patch(style)),
            (after, span.style),
        ] {
            if !text.is_empty() {
                spans.push(Span::styled(text, part_style));
            }
        }
    }
    line.spans = spans;
}

fn style_link_occurrence(
    lines: &mut [ratatui::text::Line<'static>],
    spans_by_line: &HashMap<usize, Vec<LinkSpan>>,
    first_line: usize,
    id: LinkId,
    style: Style,
) {
    for (offset, line) in lines.iter_mut().enumerate() {
        if let Some(ranges) = spans_by_line.get(&(first_line + offset)) {
            for link in ranges.iter().filter(|link| link.link_id == id) {
                apply_column_range_style(line, link.start_col, link.end_col, style);
            }
        }
    }
}

pub(super) fn render_status_bar(f: &mut Frame, app: &mut App, area: Rect) {
    let pct = app.scroll_percent();
    let bar_bg = super::status::status_bar_bg();
    app.refresh_status_cache(pct);

    f.render_widget(
        Paragraph::new(vec![app.status_line().clone()]).style(Style::default().bg(bar_bg)),
        area,
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CommentMark {
    None,
    Commented,
    Selected,
}

fn comment_marks(app: &App, scroll: usize, visible_end: usize) -> Vec<CommentMark> {
    let selection = app.comment_selection();
    let file = (!app.comments().is_empty()).then(|| app.comment_file_label());
    (scroll..visible_end)
        .map(|idx| {
            if selection.is_some_and(|(lo, hi)| (lo..=hi).contains(&idx)) {
                CommentMark::Selected
            } else if file
                .as_deref()
                .is_some_and(|file| app.is_line_commented(file, idx))
            {
                CommentMark::Commented
            } else {
                CommentMark::None
            }
        })
        .collect()
}

// Markers go in the left padding column so they never shift content or wrapping.
fn draw_comment_marks(
    f: &mut Frame,
    app: &App,
    marks: &[CommentMark],
    scroll: usize,
    x: u16,
    top: u16,
) {
    if marks.iter().all(|mark| *mark == CommentMark::None) {
        return;
    }
    let theme = app_theme();
    let buf = f.buffer_mut();
    for (row, &line_idx) in app.screen_geometry.row_lines.iter().enumerate() {
        let (symbol, fg) = match marks.get(line_idx.saturating_sub(scroll)) {
            Some(CommentMark::Selected) => ("▌", theme.ui.toc_accent),
            Some(CommentMark::Commented) => ("▎", theme.ui.status_warning_fg),
            _ => continue,
        };
        if let Some(cell) = buf.cell_mut((x, top.saturating_add(row as u16))) {
            cell.set_symbol(symbol).set_fg(fg);
        }
    }
}
