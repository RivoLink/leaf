use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn handle_comment_input_key(app: &mut App, key: &KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Esc => app.cancel_comment_input(),
        KeyCode::Char('c') if ctrl => app.cancel_comment_input(),
        KeyCode::Char('u') if ctrl => app.clear_comment_draft_text(),
        KeyCode::Enter => app.confirm_comment_input(),
        KeyCode::Backspace => app.pop_comment_char(),
        KeyCode::Char(c) if !ctrl => app.push_comment_char(c),
        _ => return false,
    }
    true
}

pub(super) fn handle_comment_mode_key(app: &mut App, key: &KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if key.code != KeyCode::Char('X') {
        app.disarm_comment_clear();
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down if !ctrl => app.move_comment_cursor_by(1),
        KeyCode::Char('k') | KeyCode::Up if !ctrl => app.move_comment_cursor_by(-1),
        KeyCode::Char('d') | KeyCode::PageDown => app.move_comment_cursor_page(true),
        KeyCode::Char('u') | KeyCode::PageUp => app.move_comment_cursor_page(false),
        KeyCode::Char('g') | KeyCode::Home if !ctrl => app.move_comment_cursor_to(0),
        KeyCode::Char('G') | KeyCode::End => app.move_comment_cursor_to(usize::MAX),
        KeyCode::Char('v') | KeyCode::Char('V') => app.toggle_comment_range(),
        KeyCode::Enter | KeyCode::Char('i') => app.begin_comment_input(),
        KeyCode::Char('x') => app.delete_comment_at_cursor(),
        KeyCode::Char('X') => app.clear_all_comments(),
        KeyCode::Char('n') => app.jump_to_comment(true),
        KeyCode::Char('N') => app.jump_to_comment(false),
        KeyCode::Char('y') | KeyCode::Char('S') if !ctrl => app.copy_comments(),
        KeyCode::Esc => {
            if !app.clear_comment_range() {
                app.exit_comment_mode();
            }
        }
        KeyCode::Char('c') if ctrl => _ = app.exit_comment_mode(),
        _ => return false,
    }
    true
}
