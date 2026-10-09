use super::*;
use crate::{
    app::{App, DiffState},
    theme::app_theme,
};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

pub(crate) fn render_diff_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = app_theme();
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.ui.content_bg)),
        area,
    );
    let content_area = inner_content_area(area);

    if app.is_diff_preview_visible() {
        render_diff_preview_panel(f, app, area, content_area);
        return;
    }

    let effective_layout = app
        .diff()
        .map_or(crate::diff::DiffLayout::Unified, |s| s.layout);
    if matches!(effective_layout, crate::diff::DiffLayout::Split) {
        render_split_panel(f, app, area, content_area);
        return;
    }

    let scroll = app.scroll();
    let viewport_height = area.height as usize;
    let visible_end = (scroll + viewport_height).min(app.total());
    let mut visible_lines = app.visible_lines(scroll, visible_end).to_vec();

    let active_file_idx = app.diff_current_file_from_scroll();
    if let Some(state) = app.diff() {
        for (fi, &top_idx) in state.file_top_idx.iter().enumerate() {
            if top_idx < scroll || top_idx >= scroll + viewport_height {
                continue;
            }
            if Some(fi) == active_file_idx {
                continue;
            }
            let rel = top_idx - scroll;
            if let Some(line) = visible_lines.get_mut(rel) {
                dim_file_top_path(line, &theme.diff);
            }
        }
    }

    if app.is_line_number_visible() {
        if let Some(state) = app.diff() {
            apply_number_gutter(&mut visible_lines, state, scroll, &theme.diff);
        }
    }

    let status_map: Option<&[Option<Vec<Span<'static>>>]> =
        app.diff().map(|s| s.file_header_status.as_slice());

    let gutter_widths: Option<(usize, usize)> = if app.is_line_number_visible() {
        app.diff().map(compute_gutter_widths)
    } else {
        None
    };
    let line_nums: Option<(&[u32], &[u32])> = app
        .diff()
        .map(|s| (s.line_old_num.as_slice(), s.line_new_num.as_slice()));

    if let Some((old_nums, new_nums)) = line_nums {
        let target = content_area.width as usize;
        for (i, line) in visible_lines.iter_mut().enumerate() {
            let idx = scroll + i;
            let o = old_nums.get(idx).copied().unwrap_or(0);
            let n = new_nums.get(idx).copied().unwrap_or(0);
            let bg: Option<Color> = if n > 0 && o == 0 {
                Some(theme.diff.add_bg)
            } else if o > 0 && n == 0 {
                Some(theme.diff.del_bg)
            } else {
                None
            };
            truncate_unified_body_row(&mut line.spans, target, bg);
        }
    }
    close_frame_right(
        &mut visible_lines,
        content_area.width,
        &theme.diff,
        &FrameDecorations {
            status_by_line: status_map,
            scroll,
            gutter_widths,
            line_nums,
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );

    if let (Some((dw_old, dw_new)), Some(state)) = (gutter_widths, app.diff()) {
        let gutter_col = 1 + dw_old + 1 + dw_new;
        for &row_idx in &state.meta_only_bottom_rows {
            if row_idx < scroll {
                continue;
            }
            let rel = row_idx - scroll;
            if let Some(line) = visible_lines.get_mut(rel) {
                strip_mid_at_col(line, gutter_col);
            }
        }
    }

    if let (Some(active), Some(state)) = (active_file_idx, app.diff()) {
        for (i, line) in visible_lines.iter_mut().enumerate() {
            if state.file_owner.get(scroll + i).copied().flatten() == Some(active) {
                recolor_frame_spans(line, theme.diff.frame_fg, active_frame_tint(&theme.diff));
            }
        }
    }

    f.render_widget(
        Paragraph::new(visible_lines).style(Style::default().bg(theme.ui.content_bg)),
        content_area,
    );

    let sticky_top_idx: Option<usize> = app
        .diff()
        .and_then(|state| compute_sticky_top_idx(state, scroll));
    if let Some(top_idx) = sticky_top_idx {
        let top_line = app.visible_lines(top_idx, top_idx + 1)[0].clone();
        let mut sticky_lines = vec![top_line];

        let sticky_fi = app
            .diff()
            .and_then(|s| s.file_owner.get(top_idx).copied().flatten());
        if sticky_fi.is_some() && sticky_fi != active_file_idx {
            if let Some(line) = sticky_lines.first_mut() {
                dim_file_top_path(line, &theme.diff);
            }
        }

        if app.is_line_number_visible() {
            if let Some(state) = app.diff() {
                apply_number_gutter(&mut sticky_lines, state, top_idx, &theme.diff);
            }
        }

        let sticky_status_map: Option<&[Option<Vec<Span<'static>>>]> =
            app.diff().map(|s| s.file_header_status.as_slice());
        close_frame_right(
            &mut sticky_lines,
            content_area.width,
            &theme.diff,
            &FrameDecorations {
                status_by_line: sticky_status_map,
                scroll: top_idx,
                gutter_widths,
                line_nums,
                auto_tee_at_gutter_col: true,
                ..Default::default()
            },
        );

        if sticky_fi.is_some() && sticky_fi == active_file_idx {
            if let Some(line) = sticky_lines.first_mut() {
                recolor_frame_spans(line, theme.diff.frame_fg, active_frame_tint(&theme.diff));
            }
        }
        let sticky_area = Rect {
            x: content_area.x,
            y: area.y,
            width: content_area.width,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(sticky_lines).style(Style::default().bg(theme.ui.content_bg)),
            sticky_area,
        );
    }

    let max_scroll = app.max_scroll();
    let (mouse_col, mouse_row) = app.mouse_position;
    let sb_x = area.x + area.width - SCROLLBAR_WIDTH;
    let on_sb_column = mouse_col >= sb_x
        && mouse_col < sb_x + SCROLLBAR_WIDTH
        && mouse_row >= area.y
        && mouse_row < area.y + area.height;
    let track_len = area.height as usize;
    let mouse_on_thumb = on_sb_column && track_len > 0 && max_scroll > 0 && {
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

pub(crate) struct BuiltSplitLinesRef<'a> {
    pub(crate) lines: &'a [Line<'static>],
    pub(crate) line_old_num_left: &'a [u32],
    pub(crate) line_new_num_left: &'a [u32],
    pub(crate) line_old_num_right: &'a [u32],
    pub(crate) line_new_num_right: &'a [u32],
    pub(crate) file_header_status: &'a [Option<Vec<Span<'static>>>],
    pub(crate) file_owner: &'a [Option<usize>],
    pub(crate) file_top_idx: &'a [usize],
    pub(crate) file_bottom_idx: &'a [usize],
    pub(crate) meta_only_frame_rows: &'a [usize],
    pub(crate) single_column_body_rows: &'a [usize],
    pub(crate) left_gutter_disabled: &'a [bool],
    pub(crate) right_gutter_disabled: &'a [bool],
}

impl<'a> BuiltSplitLinesRef<'a> {
    pub(crate) fn from_state(state: &'a DiffState) -> Self {
        Self {
            lines: state.split_lines.as_slice(),
            line_old_num_left: state.split_line_old_num_left.as_slice(),
            line_new_num_left: state.split_line_new_num_left.as_slice(),
            line_old_num_right: state.split_line_old_num_right.as_slice(),
            line_new_num_right: state.split_line_new_num_right.as_slice(),
            file_header_status: state.split_file_header_status.as_slice(),
            file_owner: state.split_file_owner.as_slice(),
            file_top_idx: state.split_file_top_idx.as_slice(),
            file_bottom_idx: state.split_file_bottom_idx.as_slice(),
            meta_only_frame_rows: state.split_meta_only_frame_rows.as_slice(),
            single_column_body_rows: state.split_single_column_body_rows.as_slice(),
            left_gutter_disabled: state.split_left_gutter_disabled.as_slice(),
            right_gutter_disabled: state.split_right_gutter_disabled.as_slice(),
        }
    }
}

pub(super) fn render_split_lines_block(
    f: &mut Frame,
    app: &App,
    area: Rect,
    content_area: Rect,
    built: BuiltSplitLinesRef<'_>,
    scroll: usize,
    gutter_widths_override: Option<(usize, usize)>,
) {
    let theme = app_theme();
    let lines_total = built.lines.len();
    let scroll = scroll.min(lines_total.saturating_sub(1));
    let viewport_height = area.height as usize;
    let visible_end = (scroll + viewport_height).min(lines_total);
    let mut visible_lines: Vec<Line<'static>> = if scroll < lines_total {
        built.lines[scroll..visible_end].to_vec()
    } else {
        Vec::new()
    };

    let active_file_idx = app.diff_current_file_from_scroll();
    for (fi, &top_idx) in built.file_top_idx.iter().enumerate() {
        if top_idx < scroll || top_idx >= scroll + viewport_height {
            continue;
        }
        if Some(fi) == active_file_idx {
            continue;
        }
        let rel = top_idx - scroll;
        if let Some(line) = visible_lines.get_mut(rel) {
            dim_file_top_path(line, &theme.diff);
        }
    }

    let mid_col = (content_area.width as usize) / 2;
    let (gutter_widths, top_tees, bottom_tees, middle_cross_tees, middle_dotted_tees) =
        if app.is_line_number_visible() {
            let widths = gutter_widths_override.unwrap_or((1, 1));
            apply_split_number_gutter_from_slices(
                &mut visible_lines,
                scroll,
                &theme.diff,
                &SplitGutterConfig {
                    old_left: built.line_old_num_left,
                    new_left: built.line_new_num_left,
                    old_right: built.line_old_num_right,
                    new_right: built.line_new_num_right,
                    widths,
                    left_disabled: built.left_gutter_disabled,
                    right_disabled: built.right_gutter_disabled,
                },
            );
            let (dw_old, dw_new) = widths;
            let left_gutter_col = 1 + dw_old + 1 + dw_new;
            let right_gutter_col = mid_col + 1 + dw_old + 1 + dw_new;
            (
                Some(widths),
                vec![mid_col],
                vec![mid_col, left_gutter_col, right_gutter_col],
                Vec::new(),
                vec![mid_col],
            )
        } else {
            (
                None,
                vec![mid_col],
                vec![mid_col],
                Vec::new(),
                vec![mid_col],
            )
        };

    apply_split_middle_pad(
        &mut visible_lines,
        &theme.diff,
        &PadContext {
            width: content_area.width,
            scroll,
            left_nums: (built.line_old_num_left, built.line_new_num_left),
            right_nums: (built.line_old_num_right, built.line_new_num_right),
            has_gutter: gutter_widths.is_some(),
            left_disabled: built.left_gutter_disabled,
        },
    );

    let status_map: Option<&[Option<Vec<Span<'static>>>]> = Some(built.file_header_status);
    let line_nums: Option<(&[u32], &[u32])> =
        Some((built.line_old_num_right, built.line_new_num_right));
    close_frame_right(
        &mut visible_lines,
        content_area.width,
        &theme.diff,
        &FrameDecorations {
            status_by_line: status_map,
            scroll,
            gutter_widths,
            line_nums,
            extra_bottom_tees: &bottom_tees,
            extra_top_tees: &top_tees,
            extra_middle_tees: &middle_cross_tees,
            extra_middle_dotted_tees: &middle_dotted_tees,
            auto_tee_at_gutter_col: false,
            single_column_body_rows: built.single_column_body_rows,
            single_column_mid_col: mid_col,
        },
    );

    if let Some((dw_old, dw_new)) = gutter_widths {
        let left_col = 1 + dw_old + 1 + dw_new;
        let right_col = mid_col + 1 + dw_old + 1 + dw_new;
        let gutter_cols = vec![left_col, right_col];
        for &row_idx in built.meta_only_frame_rows {
            if row_idx < scroll {
                continue;
            }
            let rel = row_idx - scroll;
            if let Some(line) = visible_lines.get_mut(rel) {
                strip_mid_at_cols(line, &gutter_cols);
            }
        }
    }

    if let Some((dw_old, dw_new)) = gutter_widths {
        let left_col = 1 + dw_old + 1 + dw_new;
        let right_col = mid_col + 1 + dw_old + 1 + dw_new;
        let viewport_end = scroll + visible_lines.len();
        for &idx in built
            .file_top_idx
            .iter()
            .chain(built.file_bottom_idx.iter())
        {
            if idx < scroll || idx >= viewport_end {
                continue;
            }
            let Some(line) = visible_lines.get_mut(idx - scroll) else {
                continue;
            };
            if built
                .left_gutter_disabled
                .get(idx)
                .copied()
                .unwrap_or(false)
            {
                replace_tee_at_col(line, left_col);
            }
            if built
                .right_gutter_disabled
                .get(idx)
                .copied()
                .unwrap_or(false)
            {
                replace_tee_at_col(line, right_col);
            }
        }
    }

    if let Some(active) = active_file_idx {
        for (i, line) in visible_lines.iter_mut().enumerate() {
            if built.file_owner.get(scroll + i).copied().flatten() == Some(active) {
                recolor_frame_spans(line, theme.diff.frame_fg, active_frame_tint(&theme.diff));
            }
        }
    }

    f.render_widget(
        Paragraph::new(visible_lines).style(Style::default().bg(theme.ui.content_bg)),
        content_area,
    );

    let sticky_top_idx: Option<usize> = compute_split_sticky_top_idx_inner(
        built.file_owner,
        built.file_top_idx,
        built.file_bottom_idx,
        scroll,
    );
    if let Some(top_idx) = sticky_top_idx {
        if let Some(top_line) = built.lines.get(top_idx).cloned() {
            let mut sticky_lines = vec![top_line];

            let sticky_fi = built.file_owner.get(top_idx).copied().flatten();
            if sticky_fi.is_some() && sticky_fi != active_file_idx {
                if let Some(line) = sticky_lines.first_mut() {
                    dim_file_top_path(line, &theme.diff);
                }
            }
            if let Some(widths) = gutter_widths {
                apply_split_number_gutter_from_slices(
                    &mut sticky_lines,
                    top_idx,
                    &theme.diff,
                    &SplitGutterConfig {
                        old_left: built.line_old_num_left,
                        new_left: built.line_new_num_left,
                        old_right: built.line_old_num_right,
                        new_right: built.line_new_num_right,
                        widths,
                        left_disabled: built.left_gutter_disabled,
                        right_disabled: built.right_gutter_disabled,
                    },
                );
            }
            let sticky_status_map: Option<&[Option<Vec<Span<'static>>>]> =
                Some(built.file_header_status);
            close_frame_right(
                &mut sticky_lines,
                content_area.width,
                &theme.diff,
                &FrameDecorations {
                    status_by_line: sticky_status_map,
                    scroll: top_idx,
                    gutter_widths,
                    line_nums,
                    extra_bottom_tees: &bottom_tees,
                    extra_top_tees: &top_tees,
                    extra_middle_tees: &middle_cross_tees,
                    extra_middle_dotted_tees: &middle_dotted_tees,
                    auto_tee_at_gutter_col: false,
                    single_column_body_rows: &[],
                    single_column_mid_col: 0,
                },
            );

            if sticky_fi.is_some() && sticky_fi == active_file_idx {
                if let Some(line) = sticky_lines.first_mut() {
                    recolor_frame_spans(line, theme.diff.frame_fg, active_frame_tint(&theme.diff));
                }
            }
            let sticky_area = Rect {
                x: content_area.x,
                y: area.y,
                width: content_area.width,
                height: 1,
            };
            f.render_widget(
                Paragraph::new(sticky_lines).style(Style::default().bg(theme.ui.content_bg)),
                sticky_area,
            );
        }
    }

    let max_scroll = lines_total.saturating_sub(viewport_height);
    let (mouse_col, mouse_row) = app.mouse_position;
    let sb_x = area.x + area.width - SCROLLBAR_WIDTH;
    let on_sb_column = mouse_col >= sb_x
        && mouse_col < sb_x + SCROLLBAR_WIDTH
        && mouse_row >= area.y
        && mouse_row < area.y + area.height;
    let track_len = area.height as usize;
    let mouse_on_thumb = on_sb_column && track_len > 0 && max_scroll > 0 && {
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

pub(super) fn render_split_panel(f: &mut Frame, app: &App, area: Rect, content_area: Rect) {
    let theme = app_theme();
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.ui.content_bg)),
        area,
    );
    let Some(state) = app.diff() else {
        return;
    };
    let gutter_widths = if app.is_line_number_visible() {
        Some(compute_gutter_widths(state))
    } else {
        None
    };
    let built = BuiltSplitLinesRef::from_state(state);
    render_split_lines_block(
        f,
        app,
        area,
        content_area,
        built,
        app.scroll(),
        gutter_widths,
    );
}
