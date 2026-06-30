use crate::{app::App, markdown::display_width, theme::app_theme};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

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

    if let Some(line_idx) = active_highlight_line {
        if (scroll..visible_end).contains(&line_idx) {
            if let Some((_, highlighted_line)) = app.highlighted_line_cache() {
                visible_lines[line_idx - scroll] = highlighted_line.clone();
            }
        }
    }

    if let Some((hover_line, span_index)) = app.hovered_link {
        if (scroll..visible_end).contains(&hover_line) {
            if let Some(link) = app
                .link_spans_by_line
                .get(&hover_line)
                .and_then(|spans| spans.get(span_index))
            {
                let vis_idx = hover_line - scroll;
                apply_hover_style(
                    &mut visible_lines[vis_idx],
                    link.start_col,
                    link.end_col,
                    theme.markdown.link_hover,
                );
            }
        }
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

    f.render_widget(
        Paragraph::new(visible_lines)
            .style(Style::default().bg(theme.ui.content_bg))
            .wrap(Wrap { trim: false }),
        content_area,
    );

    let (mouse_col, mouse_row) = app.mouse_position;
    let sb_x = area.x + area.width - SCROLLBAR_WIDTH;
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

fn inner_content_area(area: Rect) -> Rect {
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

fn apply_hover_style(
    line: &mut ratatui::text::Line<'static>,
    start_col: usize,
    end_col: usize,
    hover_color: Color,
) {
    let mut col = 0usize;
    for span in &mut line.spans {
        let w = display_width(span.content.as_ref());
        let span_end = col + w;
        if span_end > start_col && col < end_col {
            span.style = span.style.fg(hover_color);
        }
        col = span_end;
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
