mod content;
mod popup;
mod popup_picker;
mod status;
mod toc;

use crate::app::App;
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

    let (toc_area, content_area): (Option<Rect>, Rect) = if app.is_toc_visible() && app.has_toc() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(0)])
            .split(root[0]);
        (Some(cols[0]), cols[1])
    } else {
        (None, root[0])
    };

    if let Some(ta) = toc_area {
        toc::render_toc_panel(f, app, ta);
    } else {
        app.toc_list_area = None;
    }

    app.content_area = content_area;
    content::render_content_panel(f, app, content_area);
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
