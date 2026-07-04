use crate::{
    app::{App, EditorFlash, LinkFlash, PathKind},
    clipboard::{copy_to_clipboard, open_url},
    editor::{self, classify, open_in_editor, split_editor_cmd, EditorResult},
    markdown::display_width,
    render::{CONTENT_HORIZONTAL_PADDING, SCROLLBAR_WIDTH},
};
use anyhow::Result;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::{io, time::Instant};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

use super::DOUBLE_CLICK_THRESHOLD;

pub(super) fn handle_mouse_event(app: &mut App, mouse: MouseEvent) -> bool {
    let prev_pos = app.mouse_position;
    app.mouse_position = (mouse.column, mouse.row);
    let state_changed = if app.is_path_popup_open() {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let now = Instant::now();
                let is_double_click = app
                    .last_click
                    .map(|(c, r, t)| {
                        c == mouse.column && r == mouse.row && t.elapsed() < DOUBLE_CLICK_THRESHOLD
                    })
                    .unwrap_or(false);
                app.last_click = Some((mouse.column, mouse.row, now));
                if is_double_click {
                    if let Some(area) = app.path_popup_rel_area {
                        if is_in_rect(area, mouse.column, mouse.row) {
                            app.copy_path_relative();
                            app.last_click = None;
                            return true;
                        }
                    }
                    if let Some(area) = app.path_popup_abs_area {
                        if is_in_rect(area, mouse.column, mouse.row) {
                            app.copy_path_absolute();
                            app.last_click = None;
                            return true;
                        }
                    }
                }
                false
            }
            MouseEventKind::Moved => {
                let new_hover = app
                    .path_popup_rel_area
                    .filter(|a| is_in_rect(*a, mouse.column, mouse.row))
                    .map(|_| PathKind::Relative)
                    .or_else(|| {
                        app.path_popup_abs_area
                            .filter(|a| is_in_rect(*a, mouse.column, mouse.row))
                            .map(|_| PathKind::Absolute)
                    });
                let changed = app.path_popup_hover != new_hover;
                app.path_popup_hover = new_hover;
                changed
            }
            _ => false,
        }
    } else if app.is_popup_open() {
        if matches!(mouse.kind, MouseEventKind::Up(..)) {
            app.scrollbar_dragging = false;
        }
        false
    } else {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if mouse_in_toc_area(app, mouse.column, mouse.row) {
                    app.scroll_toc_up(super::MOUSE_SCROLL_STEP);
                    return true;
                }
                app.exit_code_select_mode();
                app.scroll_up(super::MOUSE_SCROLL_STEP);
                app.hovered_link = None;
                app.hovered_toc_idx = None;
                true
            }
            MouseEventKind::ScrollDown => {
                if mouse_in_toc_area(app, mouse.column, mouse.row) {
                    app.scroll_toc_down(super::MOUSE_SCROLL_STEP);
                    return true;
                }
                app.exit_code_select_mode();
                app.scroll_down(super::MOUSE_SCROLL_STEP);
                app.hovered_link = None;
                app.hovered_toc_idx = None;
                true
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let now = Instant::now();
                let is_double_click = app
                    .last_click
                    .map(|(c, r, t)| {
                        c == mouse.column && r == mouse.row && t.elapsed() < DOUBLE_CLICK_THRESHOLD
                    })
                    .unwrap_or(false);
                app.last_click = Some((mouse.column, mouse.row, now));

                if let Some(area) = app.toc_list_area {
                    let scroll_offset = app.toc_scroll_offset(area.height);
                    if let Some(display_idx) = toc_display_index_at(
                        area,
                        app.toc_display_entries().len(),
                        scroll_offset,
                        mouse.column,
                        mouse.row,
                    ) {
                        app.scroll_to_toc_display_line(display_idx);
                        return true;
                    }
                }

                let gutter = app.line_number_gutter_width() as u16;
                let link_hit = app.link_at_position(
                    mouse.column,
                    mouse.row,
                    CONTENT_HORIZONTAL_PADDING,
                    SCROLLBAR_WIDTH,
                    gutter,
                );
                if app.debug_input_enabled() {
                    super::debug_log(
                        true,
                        &format!(
                            "left_click link_hit={} dbl={} modifiers={:?}",
                            link_hit.is_some(),
                            is_double_click,
                            mouse.modifiers,
                        ),
                    );
                }
                if let Some(link) = link_hit {
                    let is_internal = link.url.starts_with('#');
                    if mouse.modifiers.contains(KeyModifiers::CONTROL) {
                        if is_internal {
                            if let Some(path) = app.filepath() {
                                let p = path.to_path_buf();
                                std::thread::spawn(move || {
                                    open_url(&p.display().to_string());
                                });
                            }
                        } else {
                            let url = link.url.clone();
                            std::thread::spawn(move || {
                                open_url(&url);
                            });
                        }
                        true
                    } else if is_double_click || mouse.modifiers.contains(KeyModifiers::ALT) {
                        let text = if is_internal {
                            app.filename().to_string()
                        } else {
                            link.url.clone()
                        };
                        if copy_to_clipboard(&text) {
                            app.set_link_flash(LinkFlash::Copied);
                        } else {
                            app.set_link_flash(LinkFlash::CopyFailed);
                        }
                        app.last_click = None;
                        true
                    } else {
                        false
                    }
                } else if is_on_scrollbar(app.content_area, mouse.column, mouse.row) {
                    app.scrollbar_dragging = true;
                    scrollbar_scroll_to(app, mouse.row);
                    true
                } else if is_double_click {
                    let inner_x = content_inner_x(app.content_area, gutter);
                    let block_hit = if mouse.column >= inner_x {
                        let inner_col = mouse.column - inner_x;
                        line_idx_at(app, mouse.column, mouse.row)
                            .and_then(|line_idx| app.code_block_at(line_idx, inner_col))
                    } else {
                        None
                    };
                    if let Some(block_idx) = block_hit {
                        app.code_select = Some(block_idx);
                        app.copy_code_block_at(block_idx);
                        app.last_click = None;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            MouseEventKind::Down(MouseButton::Middle | MouseButton::Right)
                if is_on_scrollbar(app.content_area, mouse.column, mouse.row) =>
            {
                app.scrollbar_dragging = true;
                scrollbar_scroll_to(app, mouse.row);
                true
            }
            MouseEventKind::Drag(..) if app.scrollbar_dragging => {
                scrollbar_scroll_to(app, mouse.row);
                true
            }
            MouseEventKind::Up(..) => {
                app.scrollbar_dragging = false;
                false
            }
            MouseEventKind::Moved if prev_pos != app.mouse_position => {
                let area = app.content_area;
                let (prev_col, prev_row) = prev_pos;
                let scrollbar_changed = is_on_scrollbar(area, prev_col, prev_row)
                    || is_on_scrollbar(area, mouse.column, mouse.row);

                let gutter = app.line_number_gutter_width() as u16;
                let new_hover = app.find_hovered_link(
                    mouse.column,
                    mouse.row,
                    CONTENT_HORIZONTAL_PADDING,
                    SCROLLBAR_WIDTH,
                    gutter,
                );
                let hover_changed = app.hovered_link != new_hover;
                if hover_changed {
                    app.hovered_link = new_hover;
                }

                let new_toc_hover = app.toc_list_area.and_then(|area| {
                    let scroll_offset = app.toc_scroll_offset(area.height);
                    toc_display_index_at(
                        area,
                        app.toc_display_entries().len(),
                        scroll_offset,
                        mouse.column,
                        mouse.row,
                    )
                });
                let toc_hover_changed = app.hovered_toc_idx != new_toc_hover;
                if toc_hover_changed {
                    app.hovered_toc_idx = new_toc_hover;
                }

                scrollbar_changed || hover_changed || toc_hover_changed
            }
            _ => false,
        }
    };
    state_changed
}

pub(super) fn handle_open_in_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> Result<()> {
    let filepath = match app.filepath() {
        Some(p) => strip_unc_prefix(p.canonicalize().unwrap_or_else(|_| p.to_path_buf())),
        None => {
            app.set_editor_flash(EditorFlash::NoFile);
            return Ok(());
        }
    };

    let editor_cmd = match app.editor_config() {
        Some(e) => {
            let visible_source_line = app.source_line_at(app.scroll());
            editor::expand_editor_placeholders(e, visible_source_line, &filepath)
        }
        None => {
            app.set_editor_flash(EditorFlash::EditorNotFound("no editor configured".into()));
            return Ok(());
        }
    };

    let emulator = editor::detect_terminal_emulator();

    if let Some(flash) =
        try_open_editor(&editor_cmd, &filepath, &emulator, terminal, app, ss, themes)?
    {
        app.set_editor_flash(flash);
        return Ok(());
    }

    if let Some(fallback) = editor::resolve_fallback_editor(&editor_cmd) {
        if let Some(flash) =
            try_open_editor(fallback, &filepath, &emulator, terminal, app, ss, themes)?
        {
            app.set_editor_flash(flash);
        }
    }

    Ok(())
}

fn try_open_editor(
    editor_cmd: &str,
    filepath: &std::path::Path,
    emulator: &editor::TerminalEmulator,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> Result<Option<EditorFlash>> {
    let kind = classify(editor_cmd);
    match open_in_editor(editor_cmd, filepath, kind, emulator) {
        Ok(EditorResult::Opened) => {
            let name = editor::binary_name(editor_cmd).to_string();
            Ok(Some(EditorFlash::Opened(name)))
        }
        Ok(EditorResult::NeedsSameTerminal) => {
            let (bin, args) = split_editor_cmd(editor_cmd);
            crossterm::terminal::disable_raw_mode()?;
            crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

            let status = std::process::Command::new(bin)
                .args(&args)
                .arg(filepath)
                .status();

            crossterm::terminal::enable_raw_mode()?;
            crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
            terminal.clear()?;
            app.reload(ss, themes);

            match status {
                Ok(s) if s.success() => {
                    let name = editor::binary_name(editor_cmd).to_string();
                    Ok(Some(EditorFlash::Opened(name)))
                }
                _ => Ok(None),
            }
        }
        Err(_) => Ok(None),
    }
}

const TOC_RIGHT_BORDER_WIDTH: u16 = 1;

fn mouse_in_toc_area(app: &App, col: u16, row: u16) -> bool {
    app.toc_list_area
        .is_some_and(|area| is_in_rect(area, col, row))
}

fn toc_display_index_at(
    area: Rect,
    entries_len: usize,
    scroll_offset: usize,
    col: u16,
    row: u16,
) -> Option<usize> {
    let inner = Rect {
        width: area.width.saturating_sub(TOC_RIGHT_BORDER_WIDTH),
        ..area
    };
    if !is_in_rect(inner, col, row) {
        return None;
    }
    let display_idx = (row - area.y) as usize + scroll_offset;
    (display_idx < entries_len).then_some(display_idx)
}

pub(super) fn is_on_scrollbar(area: Rect, col: u16, row: u16) -> bool {
    area.width > 0 && {
        let sb_x = area.x + area.width - SCROLLBAR_WIDTH;
        col >= sb_x && col < sb_x + SCROLLBAR_WIDTH && row >= area.y && row < area.y + area.height
    }
}

pub(super) fn scrollbar_scroll_to(app: &mut App, row: u16) {
    let content_top = app.content_area.y as usize;
    let content_height = app.content_area.height as usize;
    let row = row as usize;
    if row >= content_top && content_height > 1 {
        let offset = (row - content_top).min(content_height - 1);
        let max_scroll = app.max_scroll();
        let scroll_pos = offset * max_scroll / (content_height - 1);
        app.scroll_to(scroll_pos);
    }
}

fn is_in_rect(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

fn content_inner_x(area: Rect, gutter: u16) -> u16 {
    area.x + CONTENT_HORIZONTAL_PADDING + gutter
}

fn line_idx_at(app: &App, col: u16, row: u16) -> Option<usize> {
    let area = app.content_area;
    let gutter = app.line_number_gutter_width() as u16;
    let inner_x = content_inner_x(area, gutter);
    let inner_w = area
        .width
        .saturating_sub(CONTENT_HORIZONTAL_PADDING * 2)
        .saturating_sub(SCROLLBAR_WIDTH)
        .saturating_sub(gutter);
    if col < inner_x || col >= inner_x + inner_w || row < area.y || row >= area.y + area.height {
        return None;
    }
    let rel_row = (row - area.y) as usize;
    let content_width = inner_w.max(1) as usize;
    let mut visual_row = 0usize;
    let total = app.lines.len();
    for line_idx in app.scroll..total {
        let line = &app.lines[line_idx];
        let line_width: usize = line
            .spans
            .iter()
            .map(|s| display_width(s.content.as_ref()))
            .sum();
        let wrapped_lines = if line_width == 0 {
            1
        } else {
            line_width.div_ceil(content_width)
        };
        if rel_row < visual_row + wrapped_lines {
            return Some(line_idx);
        }
        visual_row += wrapped_lines;
        if visual_row > area.height as usize {
            break;
        }
    }
    None
}

fn strip_unc_prefix(path: std::path::PathBuf) -> std::path::PathBuf {
    if cfg!(target_os = "windows") {
        let s = path.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return std::path::PathBuf::from(stripped);
        }
    }
    path
}
