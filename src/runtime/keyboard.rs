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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PageDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickerPageMode {
    Browser,
    Filtered,
}

fn picker_page_direction(key: KeyEvent, mode: PickerPageMode) -> Option<PageDirection> {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::PageDown => Some(PageDirection::Down),
        KeyCode::PageUp => Some(PageDirection::Up),
        KeyCode::Char('d') if mode == PickerPageMode::Browser && !control => {
            Some(PageDirection::Down)
        }
        KeyCode::Char('u') if mode == PickerPageMode::Browser && !control => {
            Some(PageDirection::Up)
        }
        KeyCode::Char('d') if mode == PickerPageMode::Filtered && control => {
            Some(PageDirection::Down)
        }
        KeyCode::Char('u') if mode == PickerPageMode::Filtered && control => {
            Some(PageDirection::Up)
        }
        _ => None,
    }
}

#[cfg(test)]
mod picker_page_key_tests {
    use super::{
        picker_page_direction,
        PageDirection::{Down, Up},
        PickerPageMode::{Browser, Filtered},
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn letter_shortcuts_follow_picker_mode() {
        let control = KeyModifiers::CONTROL;
        let no_modifiers = KeyModifiers::NONE;
        let cases = [
            (Filtered, 'd', control, Some(Down)),
            (Filtered, 'u', control, Some(Up)),
            (Filtered, 'd', no_modifiers, None),
            (Filtered, 'u', no_modifiers, None),
            (Browser, 'd', no_modifiers, Some(Down)),
            (Browser, 'u', no_modifiers, Some(Up)),
            (Browser, 'd', control, None),
            (Browser, 'u', control, None),
        ];

        for (mode, character, modifiers, expected) in cases {
            assert_eq!(
                picker_page_direction(KeyEvent::new(KeyCode::Char(character), modifiers), mode),
                expected
            );
        }
    }

    #[test]
    fn page_keys_work_in_both_picker_modes() {
        for mode in [Browser, Filtered] {
            for (code, expected) in [(KeyCode::PageDown, Down), (KeyCode::PageUp, Up)] {
                assert_eq!(
                    picker_page_direction(KeyEvent::new(code, KeyModifiers::NONE), mode),
                    Some(expected)
                );
            }
        }
    }
}

pub(super) fn handle_key_event(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    key: KeyEvent,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> anyhow::Result<HandleResult> {
    #[cfg(unix)]
    if matches!(key.code, KeyCode::Char('z') | KeyCode::Char('Z'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        crate::terminal::TerminalSession::suspend_job(terminal, app.is_mouse_capture_enabled())?;
        return Ok(HandleResult::Continue { redraw: true });
    }

    if matches!(key.code, KeyCode::Char('m') | KeyCode::Char('M'))
        && !key.modifiers.contains(KeyModifiers::CONTROL)
    {
        let in_text_input = app.is_search_mode()
            || app.is_goto_line_mode()
            || (app.is_file_picker_open() && app.is_fuzzy_file_picker());
        if !in_text_input {
            app.exit_link_select_mode();
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
    } else if app.is_history_picker_loading() {
        let has_content = app.has_content();
        match key.code {
            KeyCode::Esc => {
                if has_content {
                    app.close_history_picker();
                } else {
                    return Ok(HandleResult::Break);
                }
            }
            KeyCode::Char('q') | KeyCode::Char('c')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if has_content {
                    app.close_history_picker();
                } else {
                    return Ok(HandleResult::Break);
                }
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
    } else if app.is_history_picker_open() {
        let has_content = app.has_content();
        let page_direction = picker_page_direction(key, PickerPageMode::Filtered);
        match key.code {
            KeyCode::Enter => {
                app.activate_history_picker_selection(ss, themes);
                state_changed = true;
            }
            KeyCode::Esc => {
                if !app.history_picker_query().is_empty() {
                    app.clear_history_picker_query();
                } else if has_content {
                    app.close_history_picker();
                } else {
                    return Ok(HandleResult::Break);
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if has_content {
                    app.close_history_picker();
                } else {
                    return Ok(HandleResult::Break);
                }
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if has_content {
                    app.close_history_picker();
                } else {
                    return Ok(HandleResult::Break);
                }
            }
            KeyCode::Down => app.move_history_picker_down(),
            KeyCode::Up => app.move_history_picker_up(),
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.move_history_picker_down()
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.move_history_picker_up()
            }
            _ if page_direction == Some(PageDirection::Down) => app.move_history_picker_page_down(),
            _ if page_direction == Some(PageDirection::Up) => app.move_history_picker_page_up(),
            KeyCode::Backspace => app.pop_history_picker_query(),
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.push_history_picker_query(c);
            }
            _ => state_changed = false,
        }
    } else if app.is_file_picker_open() {
        let has_content = app.has_content();
        let page_mode = if app.is_browser_file_picker() {
            PickerPageMode::Browser
        } else {
            PickerPageMode::Filtered
        };
        let page_direction = picker_page_direction(key, page_mode);
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
            _ if page_direction == Some(PageDirection::Down) => app.move_file_picker_page_down(),
            _ if page_direction == Some(PageDirection::Up) => app.move_file_picker_page_up(),
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

        match handle_select_mode_keys(app, &key) {
            SelectModeOutcome::Consumed => {
                return Ok(HandleResult::Continue { redraw: true });
            }
            SelectModeOutcome::Exited => mode_exited = true,
            SelectModeOutcome::Inactive => {}
        }

        match handle_shared_keys(terminal, app, &key, ss, themes)? {
            SharedOutcome::Break => return Ok(HandleResult::Break),
            SharedOutcome::Handled => {}
            SharedOutcome::NotHandled => {
                let handled = match app.mode() {
                    crate::app::AppMode::Diff => {
                        super::diff_keys::handle_diff_keys(app, &key, ss, themes)
                    }
                    crate::app::AppMode::Document => handle_document_keys(app, &key),
                };
                if !handled {
                    state_changed = false;
                }
            }
        }
        if mode_exited {
            state_changed = true;
        }
    }

    Ok(HandleResult::Continue {
        redraw: state_changed,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectModeOutcome {
    Consumed,
    Exited,
    Inactive,
}

pub(crate) fn handle_select_mode_keys(app: &mut App, key: &KeyEvent) -> SelectModeOutcome {
    if app.mode() != crate::app::AppMode::Document {
        return SelectModeOutcome::Inactive;
    }
    let exited = if app.is_link_select_mode() {
        if super::link_select_keys::handle_link_select_key(app, key) {
            return SelectModeOutcome::Consumed;
        }
        app.exit_link_select_mode()
    } else if app.is_code_select_mode() {
        if super::code_select_keys::handle_code_select_key(app, key) {
            return SelectModeOutcome::Consumed;
        }
        app.exit_code_select_mode()
    } else {
        false
    };
    if super::link_select_keys::try_link_select_entry(app, key)
        || super::code_select_keys::try_code_select_entry(app, key)
    {
        SelectModeOutcome::Consumed
    } else if exited {
        SelectModeOutcome::Exited
    } else {
        SelectModeOutcome::Inactive
    }
}

enum SharedOutcome {
    Handled,
    NotHandled,
    Break,
}

fn handle_enter_key(app: &mut App, key: &KeyEvent) -> Option<SharedOutcome> {
    if key.code != KeyCode::Enter {
        return None;
    }
    if app.has_active_search() {
        app.next_match();
        return Some(SharedOutcome::Handled);
    }
    if !app.scroll_with_enter() {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        app.scroll_up(1);
    } else {
        app.scroll_down(1);
    }
    Some(SharedOutcome::Handled)
}

fn handle_shared_keys(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    key: &KeyEvent,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> anyhow::Result<SharedOutcome> {
    if let Some(outcome) = handle_enter_key(app, key) {
        return Ok(outcome);
    }
    match key.code {
        KeyCode::Esc => {
            if app.is_diff_preview_visible() {
                let theme = crate::theme::current_syntect_theme(themes);
                app.diff_toggle_preview(ss, theme);
                return Ok(SharedOutcome::Handled);
            }
            if app.has_active_goto_line() {
                app.clear_active_goto_line();
                return Ok(SharedOutcome::Handled);
            }
            if app.has_active_search() {
                app.clear_active_search();
                return Ok(SharedOutcome::Handled);
            }
            Ok(SharedOutcome::NotHandled)
        }
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.queue_fuzzy_file_picker(app.picker_dir());
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => Ok(SharedOutcome::Break),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.is_diff_preview_visible() {
                let theme = crate::theme::current_syntect_theme(themes);
                app.diff_toggle_preview(ss, theme);
                return Ok(SharedOutcome::Handled);
            }
            if app.has_active_search() {
                app.clear_active_search();
                Ok(SharedOutcome::Handled)
            } else if app.has_active_goto_line() {
                app.clear_active_goto_line();
                Ok(SharedOutcome::Handled)
            } else {
                Ok(SharedOutcome::Break)
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.scroll_down(1);
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.scroll_up(1);
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('d') | KeyCode::PageDown => {
            app.scroll_page_down();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('u') | KeyCode::PageUp => {
            app.scroll_page_up();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.begin_goto_line();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('g') | KeyCode::Home => {
            app.scroll_top();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('G') | KeyCode::End => {
            app.scroll_bottom();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('T') => {
            app.open_theme_picker();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('E') => {
            app.open_editor_picker();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('?') => {
            app.open_help();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('w') => {
            app.toggle_watch();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if !app.is_any_picker_active() {
                app.queue_history_picker();
            }
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            reload_helper(app, ss, themes);
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('r') => {
            if app.is_diff_mode() {
                return Ok(SharedOutcome::NotHandled);
            }
            reload_helper(app, ss, themes);
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.clear_active_goto_line();
            app.begin_search();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('/') => {
            app.clear_active_goto_line();
            app.begin_search();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char(':') => {
            app.begin_goto_line();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('R') => {
            app.copy_path_to_clipboard_relative();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('A') => {
            app.copy_path_to_clipboard_absolute();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.begin_goto_line();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('l') | KeyCode::Char('L')
            if !key.modifiers.contains(KeyModifiers::ALT)
                && !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.toggle_line_numbers();
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            handle_open_in_editor(terminal, app, ss, themes)?;
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.queue_fuzzy_file_picker(app.picker_dir());
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('P') => {
            app.queue_file_picker(app.picker_dir());
            Ok(SharedOutcome::Handled)
        }
        KeyCode::Char('p') => {
            if app.mode() == crate::app::AppMode::Document {
                app.open_path_popup();
                Ok(SharedOutcome::Handled)
            } else {
                Ok(SharedOutcome::NotHandled)
            }
        }
        _ => Ok(SharedOutcome::NotHandled),
    }
}

fn reload_helper(app: &mut App, ss: &SyntaxSet, themes: &ThemeSet) {
    if app.is_diff_mode() {
        let flash = if app.refresh_diff(ss, themes) {
            crate::app::DiffFlash::Refreshed
        } else {
            crate::app::DiffFlash::RefreshFailed
        };
        app.set_diff_flash(flash);
        return;
    }
    if app.filepath().is_none() {
        let flash = app.watch_flash_for_no_file();
        app.set_watch_flash(flash);
    } else if !app.is_watch_enabled() {
        app.set_watch_flash(WatchFlash::NotActive);
    } else if !app.request_reload(ss, themes) {
        app.set_watch_flash(WatchFlash::FileNotFound);
    }
}

fn handle_document_keys(app: &mut App, key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('J') if app.can_scroll_toc() => {
            app.focus_next_top_level_toc();
            true
        }
        KeyCode::Char('K') if app.can_scroll_toc() => {
            app.focus_prev_top_level_toc();
            true
        }
        KeyCode::Char('D') if app.can_scroll_toc() => {
            app.scroll_toc_down(app.toc_half_page_step());
            true
        }
        KeyCode::Char('U') if app.can_scroll_toc() => {
            app.scroll_toc_up(app.toc_half_page_step());
            true
        }
        KeyCode::Char(' ') => {
            app.scroll_page_down();
            true
        }
        KeyCode::Char('t') => {
            app.toggle_toc();
            true
        }
        KeyCode::Char('>') if app.can_resize_side_panel() => {
            app.grow_toc_width();
            true
        }
        KeyCode::Char('<') if app.can_resize_side_panel() => {
            app.shrink_toc_width();
            true
        }
        KeyCode::Char('l')
            if key.modifiers.contains(KeyModifiers::ALT) && app.can_resize_side_panel() =>
        {
            app.grow_toc_width();
            true
        }
        KeyCode::Char('h')
            if key.modifiers.contains(KeyModifiers::ALT) && app.can_resize_side_panel() =>
        {
            app.shrink_toc_width();
            true
        }
        KeyCode::Char('n') => {
            app.next_match();
            true
        }
        KeyCode::Char('N') => {
            app.prev_match();
            true
        }
        KeyCode::Char('0') => {
            app.toggle_reverse_mode();
            false
        }
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            if let Some(n) = c.to_digit(10) {
                app.cycle_numkey(n as u8);
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use ratatui::text::Line;

    #[test]
    fn active_search_takes_priority_over_alt_enter() {
        let lines = ["zero", "needle", "two", "needle", "four", "five", "six"]
            .into_iter()
            .map(Line::from)
            .collect();
        let mut app = App::new(
            lines,
            Vec::new(),
            "test".to_string(),
            false,
            false,
            None,
            None,
        );
        app.set_scroll_with_enter(true);
        app.set_search_query("needle");
        app.run_search();
        app.scroll_to(5);

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT);
        let outcome = handle_enter_key(&mut app, &key);

        assert!(matches!(outcome, Some(SharedOutcome::Handled)));
        assert_eq!(app.scroll(), 3);
    }

    #[test]
    fn space_pages_down_in_document_mode() {
        let lines = (0..100).map(|i| Line::from(i.to_string())).collect();
        let mut app = App::new(
            lines,
            Vec::new(),
            "test".to_string(),
            false,
            false,
            None,
            None,
        );
        app.content_area = ratatui::layout::Rect::new(0, 0, 80, 10);

        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        assert!(handle_document_keys(&mut app, &key));
        assert_eq!(app.scroll(), 20);
    }
}
