use crate::app::{App, WatchFlash};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

use super::mouse::handle_open_in_editor;

pub(super) enum HandleResult {
    Continue { redraw: bool },
    Break,
}

pub(super) fn handle_key_event(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    key: KeyEvent,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> anyhow::Result<HandleResult> {
    if matches!(key.code, KeyCode::Char('m') | KeyCode::Char('M'))
        && !key.modifiers.contains(KeyModifiers::CONTROL)
    {
        let in_text_input = app.is_search_mode()
            || app.is_goto_line_mode()
            || (app.is_file_picker_open() && app.is_fuzzy_file_picker());
        if !in_text_input {
            let now_enabled = app.toggle_mouse_capture();
            if now_enabled {
                execute!(terminal.backend_mut(), EnableMouseCapture)?;
            } else {
                execute!(terminal.backend_mut(), DisableMouseCapture)?;
            }
            return Ok(HandleResult::Continue { redraw: true });
        }
    }

    let mut state_changed = true;
    if app.is_help_open() {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') => app.close_help(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.close_help();
            }
            _ => state_changed = false,
        }
    } else if app.is_picker_loading() {
        let has_content = app.has_content();
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('c')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if has_content {
                    app.cancel_picker_loading();
                } else {
                    return Ok(HandleResult::Break);
                }
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if has_content {
                    app.cancel_picker_loading();
                }
                state_changed = has_content;
            }
            KeyCode::Char('P') => {
                if has_content {
                    app.cancel_picker_loading();
                }
                state_changed = has_content;
            }
            _ => state_changed = false,
        }
    } else if app.is_picker_load_failed() {
        let has_content = app.has_content();
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('c')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if has_content {
                    app.cancel_picker_loading();
                } else {
                    return Ok(HandleResult::Break);
                }
            }
            _ => state_changed = false,
        }
    } else if app.is_file_picker_open() {
        let has_content = app.has_content();
        match key.code {
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Enter => {
                state_changed = app.activate_file_picker_selection(ss, themes);
            }
            KeyCode::Char('q') if app.is_browser_file_picker() => {
                if has_content {
                    app.close_file_picker();
                } else {
                    return Ok(HandleResult::Break);
                }
            }
            KeyCode::Char('j') | KeyCode::Down if app.is_browser_file_picker() => {
                app.move_file_picker_down()
            }
            KeyCode::Char('j')
                if key.modifiers.contains(KeyModifiers::CONTROL) && app.is_fuzzy_file_picker() =>
            {
                app.move_file_picker_down()
            }
            KeyCode::Char('k') | KeyCode::Up if app.is_browser_file_picker() => {
                app.move_file_picker_up()
            }
            KeyCode::Char('k')
                if key.modifiers.contains(KeyModifiers::CONTROL) && app.is_fuzzy_file_picker() =>
            {
                app.move_file_picker_up()
            }
            KeyCode::Down if app.is_fuzzy_file_picker() => app.move_file_picker_down(),
            KeyCode::Up if app.is_fuzzy_file_picker() => app.move_file_picker_up(),
            KeyCode::Esc => {
                if app.is_fuzzy_file_picker() && !app.file_picker_query().is_empty() {
                    app.clear_file_picker_query();
                } else if has_content {
                    app.close_file_picker();
                } else {
                    return Ok(HandleResult::Break);
                }
            }
            KeyCode::Char('h') | KeyCode::Left if app.is_browser_file_picker() => {
                state_changed = app.open_file_picker_parent();
            }
            KeyCode::Backspace if app.is_browser_file_picker() => {
                state_changed = app.open_file_picker_parent();
            }
            KeyCode::Backspace => app.pop_file_picker_query(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if has_content {
                    app.close_file_picker();
                } else {
                    return Ok(HandleResult::Break);
                }
            }
            KeyCode::Char('p') | KeyCode::Char('q')
                if key.modifiers.contains(KeyModifiers::CONTROL) && app.is_fuzzy_file_picker() =>
            {
                if has_content {
                    app.close_file_picker();
                }
                state_changed = has_content;
            }
            KeyCode::Char('P') if app.is_browser_file_picker() => {
                if has_content {
                    app.close_file_picker();
                }
                state_changed = has_content;
            }
            KeyCode::Char(c)
                if app.is_fuzzy_file_picker() && !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                app.push_file_picker_query(c);
            }
            _ => state_changed = false,
        }
    } else if app.is_theme_picker_open() {
        let mut needs_redraw = false;
        match key.code {
            KeyCode::Esc | KeyCode::Char('T') => {
                app.restore_theme_picker_preview(ss, themes);
                needs_redraw = true;
                state_changed = false;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.restore_theme_picker_preview(ss, themes);
                needs_redraw = true;
                state_changed = false;
            }
            KeyCode::Enter => app.close_theme_picker(),
            KeyCode::Char('j') | KeyCode::Down => {
                app.move_theme_picker_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.move_theme_picker_up();
            }
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                if let Some(n) = c.to_digit(10) {
                    let idx = n as usize - 1;
                    if !app.set_theme_picker_index(idx) {
                        state_changed = false;
                    }
                }
            }
            _ => state_changed = false,
        }
        if state_changed {
            if let Some(preset) = app.selected_theme_preset() {
                app.preview_theme_preset(preset, ss, themes);
            }
        }
        if needs_redraw {
            return Ok(HandleResult::Continue { redraw: true });
        }
    } else if app.is_editor_picker_open() {
        match key.code {
            KeyCode::Esc | KeyCode::Char('E') => app.cancel_editor_picker(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.cancel_editor_picker();
            }
            KeyCode::Enter => app.close_editor_picker(),
            KeyCode::Char('j') | KeyCode::Down => app.move_editor_picker_down(),
            KeyCode::Char('k') | KeyCode::Up => app.move_editor_picker_up(),
            _ => state_changed = false,
        }
    } else if app.is_path_popup_open() {
        match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('p') => app.close_path_popup(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.close_path_popup();
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                app.copy_path_relative();
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                app.copy_path_absolute();
            }
            _ => state_changed = false,
        }
    } else if app.is_goto_line_mode() {
        match key.code {
            KeyCode::Esc => app.clear_active_goto_line(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.clear_active_goto_line();
            }
            KeyCode::Enter => app.confirm_goto_line(),
            KeyCode::Backspace => app.pop_goto_draft(),
            KeyCode::Char(c) => app.push_goto_draft(c),
            _ => state_changed = false,
        }
    } else if app.is_search_mode() {
        match key.code {
            KeyCode::Esc => app.cancel_search(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.cancel_search();
            }
            KeyCode::Enter => app.confirm_search(),
            KeyCode::Backspace => app.pop_search_draft(),
            KeyCode::Char(c) => app.push_search_draft(c),
            _ => state_changed = false,
        }
    } else {
        let mut mode_exited = false;
        if app.is_code_select_mode() {
            let mode_handled = handle_code_select_key(app, &key);
            if mode_handled {
                return Ok(HandleResult::Continue { redraw: true });
            }
            mode_exited = app.exit_code_select_mode();
        } else if try_code_select_entry(app, &key) {
            return Ok(HandleResult::Continue { redraw: true });
        }
        match key.code {
            KeyCode::Esc if app.has_active_goto_line() => app.clear_active_goto_line(),
            KeyCode::Esc if app.has_active_search() => app.clear_active_search(),
            KeyCode::Enter if app.has_active_search() => app.next_match(),
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.queue_fuzzy_file_picker(app.picker_dir());
            }
            KeyCode::Char('q') => return Ok(HandleResult::Break),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if app.has_active_search() {
                    app.clear_active_search();
                } else if app.has_active_goto_line() {
                    app.clear_active_goto_line();
                } else {
                    return Ok(HandleResult::Break);
                }
            }
            KeyCode::Char('j') | KeyCode::Down => app.scroll_down(1),
            KeyCode::Char('k') | KeyCode::Up => app.scroll_up(1),
            KeyCode::Char('d') | KeyCode::PageDown => app.scroll_down(20),
            KeyCode::Char('u') | KeyCode::PageUp => app.scroll_up(20),
            KeyCode::Char('g') | KeyCode::Home => app.scroll_top(),
            KeyCode::Char('G') | KeyCode::End => app.scroll_bottom(),
            KeyCode::Char('J') if app.can_scroll_toc() => app.focus_next_top_level_toc(),
            KeyCode::Char('K') if app.can_scroll_toc() => app.focus_prev_top_level_toc(),
            KeyCode::Char('D') if app.can_scroll_toc() => {
                app.scroll_toc_down(app.toc_half_page_step());
            }
            KeyCode::Char('U') if app.can_scroll_toc() => {
                app.scroll_toc_up(app.toc_half_page_step());
            }
            KeyCode::Char('t') => app.toggle_toc(),
            KeyCode::Char('T') => {
                app.open_theme_picker();
            }
            KeyCode::Char('E') => {
                app.open_editor_picker();
            }
            KeyCode::Char('?') => {
                app.open_help();
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.toggle_watch();
            }
            KeyCode::Char('w') => {
                app.toggle_watch();
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if app.filepath().is_none() {
                    let flash = app.watch_flash_for_no_file();
                    app.set_watch_flash(flash);
                } else if !app.is_watch_enabled() {
                    app.set_watch_flash(WatchFlash::NotActive);
                } else if !app.request_reload(ss, themes) {
                    app.set_watch_flash(WatchFlash::FileNotFound);
                }
            }
            KeyCode::Char('r') => {
                if app.filepath().is_none() {
                    let flash = app.watch_flash_for_no_file();
                    app.set_watch_flash(flash);
                } else if !app.is_watch_enabled() {
                    app.set_watch_flash(WatchFlash::NotActive);
                } else if !app.request_reload(ss, themes) {
                    app.set_watch_flash(WatchFlash::FileNotFound);
                }
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.clear_active_goto_line();
                app.begin_search()
            }
            KeyCode::Char('/') => {
                app.clear_active_goto_line();
                app.begin_search()
            }
            KeyCode::Char('n') => app.next_match(),
            KeyCode::Char('N') => app.prev_match(),
            KeyCode::Char('R') => {
                app.copy_path_to_clipboard_relative();
            }
            KeyCode::Char('A') => {
                app.copy_path_to_clipboard_absolute();
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.begin_goto_line()
            }
            KeyCode::Char('l') | KeyCode::Char('L') => app.toggle_line_numbers(),
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                handle_open_in_editor(terminal, app, ss, themes)?;
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.queue_fuzzy_file_picker(app.picker_dir());
            }
            KeyCode::Char('P') => {
                app.queue_file_picker(app.picker_dir());
            }
            KeyCode::Char('p') => {
                app.open_path_popup();
            }
            KeyCode::Char('0') => {
                app.toggle_reverse_mode();
                state_changed = false;
            }
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                if let Some(n) = c.to_digit(10) {
                    app.cycle_numkey(n as u8);
                }
            }
            _ => state_changed = false,
        }
        if mode_exited {
            state_changed = true;
        }
    }

    Ok(HandleResult::Continue {
        redraw: state_changed,
    })
}

fn handle_code_select_key(app: &mut App, key: &KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('c') if ctrl => {
            app.exit_code_select_mode();
            true
        }
        KeyCode::Char('y') if ctrl => {
            app.copy_selected_code_block();
            true
        }
        KeyCode::Char('c') | KeyCode::Char('y') if !ctrl => {
            app.code_select_next();
            true
        }
        KeyCode::Char('C') | KeyCode::Char('Y') => {
            app.code_select_prev();
            true
        }
        KeyCode::Enter => {
            app.copy_selected_code_block();
            true
        }
        KeyCode::Esc => {
            app.exit_code_select_mode();
            true
        }
        _ => false,
    }
}

fn try_code_select_entry(app: &mut App, key: &KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('y') if ctrl => {
            app.copy_first_visible_code_block();
            true
        }
        KeyCode::Char('c') | KeyCode::Char('y') | KeyCode::Char('C') | KeyCode::Char('Y')
            if !ctrl =>
        {
            app.enter_code_select_mode();
            true
        }
        _ => false,
    }
}
