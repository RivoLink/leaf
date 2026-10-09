use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn handle_code_select_key(app: &mut App, key: &KeyEvent) -> bool {
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

pub(super) fn try_code_select_entry(app: &mut App, key: &KeyEvent) -> bool {
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
