use super::*;
use crate::{
    app::App,
    diff::{DiffFile, FileChange},
    theme::{app_theme, AppTheme},
};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

pub(crate) fn render_diff_preview_panel(f: &mut Frame, app: &App, area: Rect, content_area: Rect) {
    let theme = app_theme();
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.ui.content_bg)),
        area,
    );
    let Some(state) = app.diff() else {
        return;
    };
    if state.files.is_empty() {
        return;
    }
    let active_idx = state
        .tree_active_idx
        .min(state.files.len().saturating_sub(1));
    let (before_empty, after_empty) = match state.preview_cache.get(&active_idx) {
        Some(pair) => (pair.before_lines.is_empty(), pair.after_lines.is_empty()),
        None => (true, true),
    };
    if before_empty && after_empty {
        let placeholder = Line::from(Span::styled(
            "(no preview available)",
            Style::default().fg(theme.diff.frame_fg),
        ));
        let placeholder_area = Rect {
            x: content_area.x,
            y: content_area.y,
            width: content_area.width,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(placeholder).style(Style::default().bg(theme.ui.content_bg)),
            placeholder_area,
        );
        return;
    }

    let body_content_area = content_area;
    if body_content_area.width < 6 || body_content_area.height < 1 {
        return;
    }

    let pair = state.preview_cache.get(&active_idx).expect("checked above");
    let file = &state.files[active_idx];
    let (added, deleted) = state.file_counts(active_idx);

    let meta_note = match &file.change {
        FileChange::Binary => Some(" binary content not shown"),
        _ => None,
    };
    if let Some(note_text) = meta_note {
        render_preview_meta_note(
            f,
            &PreviewMetaCtx {
                theme: &theme,
                area: body_content_area,
                file,
                note_text,
                added,
                deleted,
            },
        );
        return;
    }

    let show_gutter = app.is_line_number_visible();
    let left_digit_w = pair.before_lines.len().max(1).to_string().len();
    let right_digit_w = pair.after_lines.len().max(1).to_string().len();

    let (top_line, status_spans) = build_file_top_frame_parts(file, &theme.diff, added, deleted);
    let mid_col = (body_content_area.width as usize) / 2;
    let mut top_tees: Vec<usize> = vec![mid_col];
    if show_gutter {
        if !pair.before_is_note {
            top_tees.push(1 + left_digit_w);
        }
        if !pair.after_is_note {
            top_tees.push(mid_col + 1 + right_digit_w);
        }
    }
    let mut top_row_lines = vec![top_line];
    close_frame_right(
        &mut top_row_lines,
        body_content_area.width,
        &theme.diff,
        &FrameDecorations {
            status_by_line: Some(&[Some(status_spans)]),
            extra_top_tees: &top_tees,
            ..Default::default()
        },
    );

    if let Some(line) = top_row_lines.first_mut() {
        recolor_frame_spans(line, theme.diff.frame_fg, active_frame_tint(&theme.diff));
    }
    let top_row_area = Rect {
        x: body_content_area.x,
        y: body_content_area.y,
        width: body_content_area.width,
        height: 1,
    };
    f.render_widget(
        Paragraph::new(top_row_lines).style(Style::default().bg(theme.ui.content_bg)),
        top_row_area,
    );

    if body_content_area.height < 2 {
        return;
    }
    let body_y = body_content_area.y.saturating_add(1);
    let body_h = body_content_area.height.saturating_sub(1);

    let frame_style = Style::default().fg(active_frame_tint(&theme.diff));
    let bg_fill = Style::default().bg(theme.ui.content_bg);

    let total_width = body_content_area.width as usize;
    let left_inner_w = mid_col.saturating_sub(1);
    let right_inner_w = total_width.saturating_sub(mid_col + 2);

    let scroll = app.scroll();
    let viewport_h = body_h as usize;
    let longest = pair.before_lines.len().max(pair.after_lines.len());

    let bottom_rel = longest.saturating_sub(scroll);

    let rails_rows = bottom_rel.min(viewport_h);
    let left_max = pair.before_lines.len().saturating_sub(viewport_h);
    let right_max = pair.after_lines.len().saturating_sub(viewport_h);
    let left_scroll = scroll.min(left_max) as u16;
    let right_scroll = scroll.min(right_max) as u16;

    let gutter_left_col = 1 + left_digit_w;
    let gutter_right_col = mid_col + 1 + right_digit_w;
    for row in 0..rails_rows {
        let y = body_y + row as u16;
        let rail = || Line::from(Span::styled("│", frame_style));
        let draw_rail = |f: &mut Frame, x: u16| {
            if x < body_content_area.x + body_content_area.width {
                f.render_widget(
                    Paragraph::new(rail()).style(bg_fill),
                    Rect {
                        x,
                        y,
                        width: 1,
                        height: 1,
                    },
                );
            }
        };

        draw_rail(f, body_content_area.x);
        draw_rail(
            f,
            body_content_area.x + body_content_area.width.saturating_sub(1),
        );
        if (mid_col as u16) < body_content_area.width {
            draw_rail(f, body_content_area.x + mid_col as u16);
        }

        if show_gutter {
            if !pair.before_is_note && (gutter_left_col as u16) < body_content_area.width {
                draw_rail(f, body_content_area.x + gutter_left_col as u16);
            }
            if !pair.after_is_note && (gutter_right_col as u16) < body_content_area.width {
                draw_rail(f, body_content_area.x + gutter_right_col as u16);
            }
        }
    }

    let prepare_lines = |lines: &[Line<'static>], col_inner_w: usize| -> Vec<Line<'static>> {
        let preserved_end: usize = if show_gutter { 3 } else { 1 };
        let preserved_width: usize = if show_gutter { left_digit_w + 1 + 2 } else { 2 };
        lines
            .iter()
            .map(|l| {
                let mut spans = l.spans.clone();

                let is_short_note =
                    spans.len() == 2 && spans.first().is_some_and(|s| s.content == "!");
                if !show_gutter && spans.len() >= 2 && !is_short_note {
                    spans.drain(0..2);
                }
                if col_inner_w > preserved_width + 1 && spans.len() > preserved_end {
                    let total_w: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                    if total_w > col_inner_w {
                        let bg = spans.get(preserved_end).and_then(|s| s.style.bg);
                        let budget = col_inner_w - preserved_width;
                        truncate_spans_tail(&mut spans, preserved_end, budget, bg);
                    }
                }

                let sigil_idx = preserved_end.saturating_sub(1);
                let row_bg = spans
                    .get(sigil_idx)
                    .and_then(|s| s.style.bg)
                    .or_else(|| spans.first().and_then(|s| s.style.bg));
                if let Some(bg) = row_bg {
                    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                    if used < col_inner_w {
                        let pad = col_inner_w - used;
                        spans.push(Span::styled(" ".repeat(pad), Style::default().bg(bg)));
                    }
                }
                Line::from(spans)
            })
            .collect()
    };
    let left_render = prepare_lines(&pair.before_lines, left_inner_w);
    let right_render = prepare_lines(&pair.after_lines, right_inner_w);

    let right_render = if show_gutter && left_digit_w != right_digit_w {
        let preserved_width_r = right_digit_w + 1 + 2;
        pair.after_lines
            .iter()
            .map(|l| {
                let mut spans = l.spans.clone();
                if right_inner_w > preserved_width_r + 1 && spans.len() > 3 {
                    let total_w: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                    if total_w > right_inner_w {
                        let bg = spans.get(3).and_then(|s| s.style.bg);
                        let budget = right_inner_w - preserved_width_r;
                        truncate_spans_tail(&mut spans, 3, budget, bg);
                    }
                }

                let row_bg = spans
                    .get(2)
                    .and_then(|s| s.style.bg)
                    .or_else(|| spans.first().and_then(|s| s.style.bg));
                if let Some(bg) = row_bg {
                    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                    if used < right_inner_w {
                        let pad = right_inner_w - used;
                        spans.push(Span::styled(" ".repeat(pad), Style::default().bg(bg)));
                    }
                }
                Line::from(spans)
            })
            .collect()
    } else {
        right_render
    };

    if left_inner_w > 0 {
        let left_area = Rect {
            x: body_content_area.x + 1,
            y: body_y,
            width: left_inner_w as u16,
            height: body_h,
        };
        f.render_widget(
            Paragraph::new(left_render)
                .style(Style::default().bg(theme.ui.content_bg))
                .scroll((left_scroll, 0)),
            left_area,
        );
    }
    if right_inner_w > 0 {
        let right_area = Rect {
            x: body_content_area.x + mid_col as u16 + 1,
            y: body_y,
            width: right_inner_w as u16,
            height: body_h,
        };
        f.render_widget(
            Paragraph::new(right_render)
                .style(Style::default().bg(theme.ui.content_bg))
                .scroll((right_scroll, 0)),
            right_area,
        );
    }

    if bottom_rel < viewport_h {
        let bottom_y = body_y + bottom_rel as u16;
        let width = body_content_area.width as usize;
        let mut bottom_spans: Vec<Span<'static>> = Vec::with_capacity(5);
        let left_corner_fill = mid_col.saturating_sub(1);
        bottom_spans.push(Span::styled(
            format!("└{}", "─".repeat(left_corner_fill)),
            frame_style,
        ));
        bottom_spans.push(Span::styled("┴", frame_style));
        let right_fill = width.saturating_sub(mid_col + 2);
        bottom_spans.push(Span::styled("─".repeat(right_fill), frame_style));
        bottom_spans.push(Span::styled("┘", frame_style));
        let bottom_line = Line::from(bottom_spans);
        let mut bottom_lines = vec![bottom_line];
        let mut bottom_tees: Vec<usize> = Vec::new();
        if show_gutter {
            if !pair.before_is_note {
                bottom_tees.push(1 + left_digit_w);
            }
            if !pair.after_is_note {
                bottom_tees.push(mid_col + 1 + right_digit_w);
            }
        }
        if !bottom_tees.is_empty() {
            if let Some(line) = bottom_lines.first_mut() {
                let mut chars: Vec<char> =
                    line.spans.iter().flat_map(|s| s.content.chars()).collect();
                for &col in &bottom_tees {
                    if col < chars.len() {
                        chars[col] = '┴';
                    }
                }
                *line = Line::from(Span::styled(
                    chars.into_iter().collect::<String>(),
                    frame_style,
                ));
            }
        }
        let bottom_area = Rect {
            x: body_content_area.x,
            y: bottom_y,
            width: body_content_area.width,
            height: 1,
        };
        f.render_widget(Paragraph::new(bottom_lines).style(bg_fill), bottom_area);
    }

    let max_scroll = app.max_scroll();
    #[allow(clippy::manual_checked_ops)]
    if max_scroll > 0 {
        let (mouse_col, mouse_row) = app.mouse_position;
        let sb_x = area.x + area.width - SCROLLBAR_WIDTH;
        let on_sb_column = mouse_col >= sb_x
            && mouse_col < sb_x + SCROLLBAR_WIDTH
            && mouse_row >= area.y
            && mouse_row < area.y + area.height;
        let track_len = area.height as usize;
        let mouse_on_thumb = on_sb_column && track_len > 0 && {
            let thumb_size = (track_len * track_len / max_scroll).max(1).min(track_len);
            let max_offset = track_len.saturating_sub(thumb_size);
            let thumb_offset = scroll * max_offset / max_scroll;
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
        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll);
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

struct PreviewMetaCtx<'a> {
    theme: &'a AppTheme,
    area: Rect,
    file: &'a DiffFile,
    note_text: &'a str,
    added: usize,
    deleted: usize,
}

fn render_preview_meta_note(f: &mut Frame, ctx: &PreviewMetaCtx<'_>) {
    let diff = &ctx.theme.diff;
    let (mut top_line, status_spans) =
        build_file_top_frame_parts(ctx.file, diff, ctx.added, ctx.deleted);
    let mut top_row_lines = vec![top_line.clone()];
    close_frame_right(
        &mut top_row_lines,
        ctx.area.width,
        diff,
        &FrameDecorations {
            status_by_line: Some(&[Some(status_spans)]),
            ..Default::default()
        },
    );
    if let Some(line) = top_row_lines.first_mut() {
        recolor_frame_spans(line, diff.frame_fg, active_frame_tint(diff));
        top_line = line.clone();
    }

    let mut body_lines = vec![build_bordered_note(diff, ctx.note_text.to_string())];
    close_frame_right(
        &mut body_lines,
        ctx.area.width,
        diff,
        &FrameDecorations::default(),
    );
    let body_line = body_lines.first().cloned().unwrap_or_default();

    let mut bottom_lines = vec![build_file_bottom_frame(diff)];
    close_frame_right(
        &mut bottom_lines,
        ctx.area.width,
        diff,
        &FrameDecorations::default(),
    );
    if let Some(line) = bottom_lines.first_mut() {
        recolor_frame_spans(line, diff.frame_fg, active_frame_tint(diff));
    }
    let bottom_line = bottom_lines.first().cloned().unwrap_or_default();

    f.render_widget(
        Paragraph::new(vec![top_line, body_line, bottom_line])
            .style(Style::default().bg(ctx.theme.ui.content_bg)),
        ctx.area,
    );
}
