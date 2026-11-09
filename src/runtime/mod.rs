mod code_select_keys;
mod diff_keys;
mod keyboard;
mod link_select_keys;
mod mouse;

use crate::{
    app::{App, AppMode, FileChange, COPY_HIGHLIGHT_DURATION_MS, FLASH_DURATION_MS},
    render::{ui, CONTENT_HORIZONTAL_PADDING, SCROLLBAR_WIDTH},
};
use anyhow::Result;
use crossterm::event::{self, poll, Event, KeyEventKind, MouseEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    fs::OpenOptions,
    io,
    io::Write,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

use keyboard::{handle_key_event, HandleResult};
#[cfg(test)]
pub(crate) use keyboard::{handle_select_mode_keys, SelectModeOutcome};
#[cfg(test)]
pub(crate) use link_select_keys::handle_link_select_key_with;
#[cfg(test)]
pub(crate) use mouse::handle_mouse_event_with;

const EDITOR_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const WATCH_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const CONFIG_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const LINK_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const CODE_BLOCK_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const PATH_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const PATH_COPY_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const HISTORY_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const COPY_HIGHLIGHT_DURATION: Duration = Duration::from_millis(COPY_HIGHLIGHT_DURATION_MS);
const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(400);
const MOUSE_SCROLL_STEP: usize = 3;

pub(crate) fn should_handle_key(kind: KeyEventKind) -> bool {
    !matches!(kind, KeyEventKind::Release)
}

fn open_url_in_background(url: &str) {
    let url = url.to_owned();
    std::thread::spawn(move || {
        crate::clipboard::open_url(&url);
    });
}

pub(crate) fn debug_log(enabled: bool, message: &str) {
    if !enabled {
        return;
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("leaf-debug.log")
    {
        let _ = writeln!(file, "[{timestamp}] {message}");
    }
}

pub(crate) fn prepare_initial_picker_state(
    area_width: u16,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> Result<()> {
    debug_log(
        app.debug_input_enabled(),
        &format!("prepare_initial_picker_state start area_width={area_width}"),
    );
    sync_render_width_for_app(area_width, app, ss, themes);
    if app.has_pending_picker() && !app.is_picker_loading() {
        let _ = app.start_pending_picker_loading();
    }
    debug_log(
        app.debug_input_enabled(),
        &format!(
            "prepare_initial_picker_state end picker_loading={} pending_picker={}",
            app.is_picker_loading(),
            app.has_pending_picker()
        ),
    );
    Ok(())
}

pub(crate) fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
    initial_draw_done: bool,
) -> Result<()> {
    const WATCH_INTERVAL: Duration = Duration::from_millis(250);
    const FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
    const RESIZE_DEBOUNCE: Duration = Duration::from_millis(120);
    const PICKER_LOAD_POLL_INTERVAL: Duration = Duration::from_millis(50);
    let mut needs_redraw = !initial_draw_done;
    let mut pending_resize: Option<Instant> = None;
    let mut last_title_filename: Option<String> = app.title_filename().map(str::to_string);
    sync_render_width(terminal, app, ss, themes)?;

    loop {
        if app.has_pending_picker() && !app.is_picker_loading() {
            let _ = app.start_pending_picker_loading();
            needs_redraw = true;
        }
        if app.poll_picker_loading() {
            needs_redraw = true;
        }
        if app.poll_history_errors() {
            needs_redraw = true;
        }

        let current_title_filename = app.title_filename();
        if current_title_filename != last_title_filename.as_deref() {
            crate::terminal::set_tab_title(
                current_title_filename,
                app.tab_title_max_filename_len(),
            );
            last_title_filename = current_title_filename.map(str::to_string);
        }

        if needs_redraw {
            terminal.draw(|f| ui(f, app))?;
            needs_redraw = false;
        }

        let flash_timeout = app
            .reload_flash_started()
            .and_then(|started| FLASH_DURATION.checked_sub(started.elapsed()));
        let editor_flash_timeout = app
            .editor_flash()
            .and_then(|(_, started)| EDITOR_FLASH_DURATION.checked_sub(started.elapsed()));
        let watch_flash_timeout = app
            .watch_flash()
            .and_then(|(_, started)| WATCH_FLASH_DURATION.checked_sub(started.elapsed()));
        let config_flash_timeout = app
            .config_flash()
            .and_then(|(_, started)| CONFIG_FLASH_DURATION.checked_sub(started.elapsed()));
        let link_flash_timeout = app
            .link_flash()
            .and_then(|(_, started)| LINK_FLASH_DURATION.checked_sub(started.elapsed()));
        let code_block_flash_timeout = app
            .code_block_flash()
            .and_then(|(_, started)| CODE_BLOCK_FLASH_DURATION.checked_sub(started.elapsed()));
        let resize_timeout =
            pending_resize.and_then(|started| RESIZE_DEBOUNCE.checked_sub(started.elapsed()));
        let poll_timeout = [
            if app.is_watch_enabled() {
                Some(WATCH_INTERVAL)
            } else {
                None
            },
            if app.is_picker_loading() {
                Some(PICKER_LOAD_POLL_INTERVAL)
            } else {
                None
            },
            flash_timeout,
            editor_flash_timeout,
            watch_flash_timeout,
            config_flash_timeout,
            link_flash_timeout,
            code_block_flash_timeout,
            app.path_flash()
                .and_then(|(_, started)| PATH_FLASH_DURATION.checked_sub(started.elapsed())),
            app.path_copy_flash().and_then(|(_, _, started)| {
                PATH_COPY_FLASH_DURATION.checked_sub(started.elapsed())
            }),
            app.history_flash()
                .and_then(|(_, started)| HISTORY_FLASH_DURATION.checked_sub(started.elapsed())),
            app.diff_flash()
                .and_then(|(_, started)| FLASH_DURATION.checked_sub(started.elapsed())),
            app.soft_break_flash()
                .and_then(|(_, started)| FLASH_DURATION.checked_sub(started.elapsed())),
            app.copy_highlight()
                .and_then(|(_, started)| COPY_HIGHLIGHT_DURATION.checked_sub(started.elapsed())),
            resize_timeout,
        ]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(Duration::MAX);

        let event_available = if poll_timeout == Duration::MAX {
            true
        } else {
            poll(poll_timeout)?
        };

        if event_available {
            match event::read()? {
                Event::Key(key) => {
                    if app.debug_input_enabled() {
                        debug_log(
                            true,
                            &format!(
                                "key_event kind={:?} code={:?} modifiers={:?} search_mode={} query={:?} draft={:?} matches={} idx={}",
                                key.kind,
                                key.code,
                                key.modifiers,
                                app.is_search_mode(),
                                app.search_query(),
                                app.search_draft(),
                                app.search_match_count(),
                                app.search_index()
                            ),
                        );
                    }
                    if !should_handle_key(key.kind) {
                        continue;
                    }
                    match handle_key_event(terminal, app, key, ss, themes)? {
                        HandleResult::Break => break,
                        HandleResult::Continue { redraw } => {
                            if redraw {
                                needs_redraw = true;
                            }
                        }
                    }
                    if sync_render_width(terminal, app, ss, themes)? {
                        needs_redraw = true;
                    }
                }
                Event::Mouse(mouse) => {
                    if app.debug_input_enabled() {
                        debug_log(
                            true,
                            &format!(
                                "mouse_event kind={:?} col={} row={} modifiers={:?}",
                                mouse.kind, mouse.column, mouse.row, mouse.modifiers
                            ),
                        );
                    }
                    let toc_dragged =
                        app.toc_dragging && matches!(mouse.kind, MouseEventKind::Drag(_));
                    let toc_drag_ended =
                        app.toc_dragging && matches!(mouse.kind, MouseEventKind::Up(_));
                    if mouse::handle_mouse_event(app, mouse) {
                        needs_redraw = true;
                    }
                    if toc_dragged {
                        sync_render_width(terminal, app, ss, themes)?;
                    }
                    if toc_drag_ended {
                        pending_resize = None;
                        sync_render_width(terminal, app, ss, themes)?;
                    }
                }
                Event::Resize(_, _) => {
                    needs_redraw |= app.exit_code_select_mode() | app.exit_link_select_mode();
                    pending_resize = Some(Instant::now());
                }
                _ => {}
            }
        }

        if pending_resize.is_some_and(|started| started.elapsed() >= RESIZE_DEBOUNCE) {
            pending_resize = None;
            sync_render_width(terminal, app, ss, themes)?;
            needs_redraw = true;
        }

        if app.is_watch_enabled() {
            let file_ok = app.filepath().is_some_and(|p| p.exists());
            if !file_ok && !app.is_watch_error() {
                app.set_watch_error(true);
                needs_redraw = true;
            } else if file_ok && app.is_watch_error() {
                app.set_watch_error(false);
                needs_redraw = true;
            }
            if file_ok {
                if let Some(change) = app.check_modified() {
                    std::thread::sleep(Duration::from_millis(50));
                    if app.reload(ss, themes) {
                        app.set_last_file_state(match change {
                            FileChange::Metadata(state) | FileChange::Content(state) => state,
                        });
                        needs_redraw = true;
                    }
                }
            }
            if let Some(t) = app.reload_flash_started() {
                if t.elapsed() >= FLASH_DURATION {
                    app.clear_reload_flash();
                    needs_redraw = true;
                }
            }
        }

        if let Some((_, started)) = app.editor_flash() {
            if started.elapsed() >= EDITOR_FLASH_DURATION {
                app.clear_editor_flash();
                needs_redraw = true;
            }
        }

        if let Some((_, started)) = app.watch_flash() {
            if started.elapsed() >= WATCH_FLASH_DURATION {
                app.clear_watch_flash();
                needs_redraw = true;
            }
        }

        if let Some((_, started)) = app.config_flash() {
            if started.elapsed() >= CONFIG_FLASH_DURATION {
                app.clear_config_flash();
                needs_redraw = true;
            }
        }

        if let Some((_, started)) = app.link_flash() {
            if started.elapsed() >= LINK_FLASH_DURATION {
                app.clear_link_flash();
                needs_redraw = true;
            }
        }

        if let Some((_, started)) = app.path_flash() {
            if started.elapsed() >= PATH_FLASH_DURATION {
                app.clear_path_flash();
                needs_redraw = true;
            }
        }

        if let Some((_, _, started)) = app.path_copy_flash() {
            if started.elapsed() >= PATH_COPY_FLASH_DURATION {
                app.clear_path_copy_flash();
                needs_redraw = true;
            }
        }

        if let Some((_, started)) = app.history_flash() {
            if started.elapsed() >= HISTORY_FLASH_DURATION {
                app.clear_history_flash();
                needs_redraw = true;
            }
        }

        if let Some((_, started)) = app.diff_flash() {
            if started.elapsed() >= FLASH_DURATION {
                app.clear_diff_flash();
                needs_redraw = true;
            }
        }

        if let Some((_, started)) = app.soft_break_flash() {
            if started.elapsed() >= FLASH_DURATION {
                app.clear_soft_break_flash();
                needs_redraw = true;
            }
        }

        if let Some((_, started)) = app.copy_highlight() {
            if started.elapsed() >= COPY_HIGHLIGHT_DURATION {
                app.clear_copy_highlight();
                needs_redraw = true;
            }
        }
    }
    Ok(())
}

fn sync_render_width(
    terminal: &Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> Result<bool> {
    let area = terminal.size()?;
    Ok(sync_render_width_for_app(area.width, app, ss, themes))
}

pub(crate) fn sync_render_width_for_app(
    area_width: u16,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> bool {
    let render_width = effective_render_width_for_app(area_width, app);
    app.sync_render_width(render_width, ss, themes)
}

fn effective_render_width_for_app(area_width: u16, app: &App) -> usize {
    let content_width = if app.mode() == AppMode::Document && app.is_toc_visible() && app.has_toc()
    {
        usize::from(area_width.saturating_sub(app.toc_width_cells(area_width)))
    } else {
        usize::from(area_width)
    };
    let gutter = app.line_number_gutter_width();
    let effective_width = content_width
        .saturating_sub(CONTENT_HORIZONTAL_PADDING as usize * 2)
        .saturating_sub(SCROLLBAR_WIDTH as usize)
        .saturating_sub(gutter);
    effective_width.min(app.max_width().unwrap_or(usize::MAX))
}

#[cfg(test)]
mod tests {
    use super::effective_render_width_for_app;
    use crate::{
        app::{App, AppMode},
        markdown::toc::TocEntry,
        picker_width::PickerWidthSpec,
    };

    fn app_with_toc() -> App {
        App::new(
            Vec::new(),
            vec![TocEntry {
                level: 1,
                title: "Heading".to_string(),
                line: 0,
            }],
            "test".to_string(),
            false,
            false,
            None,
            None,
        )
    }

    #[test]
    fn render_width_uses_effective_toc_width_below_and_above_thirty_cells() {
        let mut app = app_with_toc();
        app.toggle_toc();

        app.set_toc_width(PickerWidthSpec::Cells(24));
        assert_eq!(effective_render_width_for_app(100, &app), 72);

        app.set_toc_width(PickerWidthSpec::Cells(40));
        assert_eq!(effective_render_width_for_app(100, &app), 56);
    }

    #[test]
    fn render_width_does_not_reserve_toc_when_hidden_or_in_diff_mode() {
        let mut app = app_with_toc();
        app.set_toc_width(PickerWidthSpec::Cells(40));
        assert_eq!(effective_render_width_for_app(100, &app), 97);

        app.toggle_toc();
        app.set_mode(AppMode::Diff);
        assert_eq!(effective_render_width_for_app(100, &app), 97);
    }
}
