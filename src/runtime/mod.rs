mod keyboard;
mod mouse;

use crate::{
    app::{App, FileChange, FLASH_DURATION_MS},
    render::{ui, CONTENT_HORIZONTAL_PADDING, SCROLLBAR_WIDTH},
};
use anyhow::Result;
use crossterm::event::{self, poll, Event, KeyEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    fs::OpenOptions,
    io,
    io::Write,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

use keyboard::{handle_key_event, HandleResult};

const EDITOR_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const WATCH_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const CONFIG_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const LINK_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const CODE_BLOCK_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const PATH_FLASH_DURATION: Duration = Duration::from_millis(FLASH_DURATION_MS);
const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(400);
const MOUSE_SCROLL_STEP: usize = 3;

pub(crate) fn should_handle_key(kind: KeyEventKind) -> bool {
    !matches!(kind, KeyEventKind::Release)
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
    area_width: usize,
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
    sync_render_width(terminal, app, ss, themes)?;

    loop {
        if app.has_pending_picker() && !app.is_picker_loading() {
            let _ = app.start_pending_picker_loading();
            needs_redraw = true;
        }
        if app.poll_picker_loading() {
            needs_redraw = true;
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
                    debug_log(
                        app.debug_input_enabled(),
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
                    if mouse::handle_mouse_event(app, mouse) {
                        needs_redraw = true;
                    }
                }
                Event::Resize(_, _) => {
                    if app.exit_code_select_mode() {
                        needs_redraw = true;
                    }
                    pending_resize = Some(Instant::now());
                }
                _ => {}
            }
        }

        if pending_resize
            .map(|started| started.elapsed() >= RESIZE_DEBOUNCE)
            .unwrap_or(false)
        {
            pending_resize = None;
            sync_render_width(terminal, app, ss, themes)?;
            needs_redraw = true;
        }

        if app.is_watch_enabled() {
            let file_ok = app.filepath().map(|p| p.exists()).unwrap_or(false);
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
    Ok(sync_render_width_for_app(
        area.width as usize,
        app,
        ss,
        themes,
    ))
}

fn sync_render_width_for_app(
    area_width: usize,
    app: &mut App,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> bool {
    let content_width = if app.is_toc_visible() && app.has_toc() {
        area_width.saturating_sub(30)
    } else {
        area_width
    };
    let gutter = app.line_number_gutter_width();
    let effective_width = content_width
        .saturating_sub(CONTENT_HORIZONTAL_PADDING as usize * 2)
        .saturating_sub(SCROLLBAR_WIDTH as usize)
        .saturating_sub(gutter);
    let capped_width = effective_width.min(app.max_width().unwrap_or(usize::MAX));
    app.sync_render_width(capped_width, ss, themes)
}
