mod content;
mod diff;
mod diff_tree;
mod popup;
mod popup_picker;
mod status;
mod toc;

use crate::app::{App, AppMode};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use diff::{
    apply_number_gutter, close_frame_right, compute_sticky_top_idx, truncate_unified_body_row,
    FrameDecorations,
};
pub(crate) use diff::{
    build_diff_lines, build_split_lines, mode_change_label, resolve_file_syntax, syntect_to_color,
    BuiltSplitLines,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

#[cfg(test)]
pub(crate) use popup::wrap_path_lines;
pub(crate) use status::build_status_bar;
pub(crate) use toc::{build_toc_line_with_index, toc_header_line};

pub(crate) const CONTENT_HORIZONTAL_PADDING: u16 = 1;
pub(crate) const SCROLLBAR_WIDTH: u16 = 1;

pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let mode = app.mode();

    let preview_active = mode == AppMode::Diff && app.is_diff_preview_visible();
    let diff_tree_visible =
        mode == AppMode::Diff && !preview_active && app.diff().is_some_and(|d| d.tree_visible);
    let toc_active = mode == AppMode::Document && app.is_toc_visible() && app.has_toc();

    let (side_area, content_area): (Option<Rect>, Rect) = if toc_active {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(0)])
            .split(root[0]);
        (Some(cols[0]), cols[1])
    } else if diff_tree_visible {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(diff_tree::DIFF_TREE_WIDTH),
                Constraint::Min(0),
            ])
            .split(root[0]);
        (Some(cols[0]), cols[1])
    } else {
        (None, root[0])
    };

    if toc_active {
        if let Some(ta) = side_area {
            toc::render_toc_panel(f, app, ta);
        }
    } else {
        app.toc_list_area = None;
        if diff_tree_visible {
            if let Some(ta) = side_area {
                diff_tree::render_diff_tree_panel(f, app, ta);
            }
        }
    }

    if !diff_tree_visible {
        app.diff_tree_list_area = None;
    }

    app.content_area = content_area;
    match mode {
        AppMode::Document => content::render_content_panel(f, app, content_area),
        AppMode::Diff => diff::render_diff_panel(f, app, content_area),
    }
    content::render_status_bar(f, app, root[1]);

    if app.is_help_open() {
        popup::render_help_popup(f, app);
    } else if app.is_history_picker_loading() {
        popup_picker::render_history_loading_popup(f, app);
    } else if app.is_history_picker_open() {
        popup_picker::render_history_popup(f, app);
    } else if app.is_picker_loading() || app.is_picker_load_failed() {
        popup_picker::render_picker_loading_popup(f, app);
    } else if app.is_file_picker_open() {
        popup_picker::render_file_popup(f, app);
    } else if app.is_theme_picker_open() {
        popup::render_theme_popup(f, app);
    } else if app.is_editor_picker_open() {
        popup_picker::render_editor_popup(f, app);
    } else if app.is_path_popup_open() {
        popup::render_path_popup(f, app);
    }
}

pub(super) fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let popup_width = width.min(area.width.saturating_sub(2)).max(1);
    let popup_height = height.min(area.height.saturating_sub(2)).max(1);
    Rect {
        x: area.x + area.width.saturating_sub(popup_width) / 2,
        y: area.y + area.height.saturating_sub(popup_height) / 2,
        width: popup_width,
        height: popup_height,
    }
}
