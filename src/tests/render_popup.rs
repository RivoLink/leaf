use super::common::app_helpers::{draw_app, draw_on, markdown_app};
use super::lock_theme_test_state;
use crate::app::App;
use crate::render::{centered_rect, HELP_DOCUMENT_POPUP_SIZE, THEME_POPUP_SIZE};
use ratatui::{
    backend::TestBackend,
    buffer::{Buffer, CellWidth},
    layout::Rect,
    Terminal,
};

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

fn cjk_app() -> App {
    let row_even = "砂糖的数据库需在里共享给连接后才能读写未共享时报错".repeat(3);
    let row_odd = format!("a{row_even}");
    let source = (0..HEIGHT)
        .map(|i| if i % 2 == 0 { &row_even } else { &row_odd }.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    markdown_app(&source, WIDTH as usize, "cjk.md", None)
}

fn popup_area((width, height): (u16, u16)) -> Rect {
    centered_rect(width, height, Rect::new(0, 0, WIDTH, HEIGHT))
}

fn assert_left_border_intact(buffer: &Buffer, popup: Rect) {
    for y in popup.y..popup.bottom() {
        let symbol = buffer[(popup.x, y)].symbol();
        assert!(
            matches!(symbol, "│" | "┌" | "└"),
            "row {y}: expected popup left border at column {}, got {symbol:?}",
            popup.x
        );
    }
}

#[test]
fn theme_popup_left_border_survives_wide_chars_underneath() {
    let _guard = lock_theme_test_state();
    let mut app = cjk_app();
    app.open_theme_picker();

    let buffer = draw_app(&mut app, WIDTH, HEIGHT);

    let popup = popup_area(THEME_POPUP_SIZE);
    assert_left_border_intact(&buffer, popup);
}

#[test]
fn closing_theme_popup_restores_wide_chars_left_of_it() {
    let _guard = lock_theme_test_state();
    let mut app = cjk_app();
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();

    let before = draw_on(&mut terminal, &mut app);
    app.open_theme_picker();
    draw_on(&mut terminal, &mut app);
    app.close_theme_picker();
    let after = draw_on(&mut terminal, &mut app);

    let popup = popup_area(THEME_POPUP_SIZE);
    let left_of_popup = |buffer: &Buffer| -> Vec<String> {
        (popup.y..popup.bottom())
            .map(|y| buffer[(popup.x - 1, y)].symbol().to_string())
            .collect()
    };
    assert!(left_of_popup(&before).iter().any(|s| s.cell_width() > 1));
    assert_eq!(left_of_popup(&after), left_of_popup(&before));
}

#[test]
fn help_popup_left_border_survives_wide_chars_underneath() {
    let mut app = cjk_app();
    app.open_help();

    let buffer = draw_app(&mut app, WIDTH, HEIGHT);

    let popup = popup_area(HELP_DOCUMENT_POPUP_SIZE);
    assert_left_border_intact(&buffer, popup);
}
