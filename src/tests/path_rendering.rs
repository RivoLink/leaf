use super::common::render_helpers::*;
use crate::app::App;
use crate::wrap_path_lines;
use ratatui::style::Style;

#[test]
fn wrap_path_lines_short_path_fits_single_line() {
    let s = Style::default();
    let lines = wrap_path_lines("Relative: ", "src/main.rs", 74, s, s);
    assert_eq!(lines.len(), 1);
    assert_eq!(plain(&lines), vec!["Relative: src/main.rs"]);
}

#[test]
fn wrap_path_lines_long_path_wraps_with_indent() {
    let s = Style::default();
    let label = "Absolute: ";
    let path = "a".repeat(80);
    let lines = wrap_path_lines(label, &path, 30, s, s);
    let text = plain(&lines);
    assert!(lines.len() > 1);
    assert!(text[0].starts_with("Absolute: "));
    for continuation in &text[1..] {
        assert!(
            continuation.starts_with("          "),
            "continuation should be indented by label width"
        );
    }
}

#[test]
fn wrap_path_lines_continuation_aligned_with_value_start() {
    let s = Style::default();
    let label = "Relative: ";
    let path = "x".repeat(100);
    let lines = wrap_path_lines(label, &path, 40, s, s);
    let text = plain(&lines);
    let value_width = 40 - label.len();
    assert_eq!(&text[0], &format!("Relative: {}", &path[..value_width]));
    assert_eq!(
        &text[1],
        &format!(
            "{}{}",
            " ".repeat(label.len()),
            &path[value_width..value_width * 2]
        )
    );
}

#[test]
fn wrap_path_lines_exact_fit_no_wrap() {
    let s = Style::default();
    let label = "Test: ";
    let path = "x".repeat(74 - label.len());
    let lines = wrap_path_lines(label, &path, 74, s, s);
    assert_eq!(lines.len(), 1);
}

#[test]
fn wrap_path_lines_one_char_over_wraps() {
    let s = Style::default();
    let label = "Test: ";
    let path = "x".repeat(74 - label.len() + 1);
    let lines = wrap_path_lines(label, &path, 74, s, s);
    assert_eq!(lines.len(), 2);
}

#[test]
fn path_popup_copy_status_uses_reserved_row_without_moving_paths() {
    use crate::app::{PathKind, FLASH_DURATION_MS};
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};
    use std::time::{Duration, Instant};

    fn draw(app: &mut App, width: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, 30)).unwrap();
        terminal.draw(|f| crate::render::ui(f, app)).unwrap();
        terminal.backend().buffer().clone()
    }

    let _guard = super::lock_theme_test_state();
    for width in [80, 60] {
        for length in [55, 56, 63, 64, 65] {
            let mut app = App::new(
                vec![],
                vec![],
                "test".to_string(),
                false,
                false,
                Some(std::path::PathBuf::from("x".repeat(length))),
                None,
            );
            app.open_path_popup();
            let idle = draw(&mut app, width);
            let relative_area = app.path_popup_rel_area.unwrap();
            let absolute_area = app.path_popup_abs_area.unwrap();
            let status_y = absolute_area.bottom();

            for (target, copied_message) in [
                (PathKind::Relative, "Relative path copied to clipboard"),
                (PathKind::Absolute, "Absolute path copied to clipboard"),
            ] {
                for success in [true, false] {
                    app.path_copy_flash = Some((target.clone(), success, Instant::now()));
                    let active = draw(&mut app, width);
                    let expected = if success {
                        copied_message.to_string()
                    } else {
                        match std::env::consts::OS {
                            "macos" => "Copy failed: pbcopy not found",
                            "windows" => "Copy failed: clip.exe not found",
                            _ => "Copy failed: install xclip or wl-clipboard",
                        }
                        .to_string()
                    };
                    let status: String = (absolute_area.x..absolute_area.right())
                        .map(|x| active.cell((x, status_y)).unwrap().symbol())
                        .collect();
                    assert_eq!(
                        status.trim_end(),
                        expected,
                        "width={width}, length={length}"
                    );
                    assert_eq!(app.path_popup_rel_area, Some(relative_area));
                    assert_eq!(app.path_popup_abs_area, Some(absolute_area));
                    for y in 0..active.area.height {
                        if y != status_y {
                            for x in 0..active.area.width {
                                assert_eq!(active.cell((x, y)), idle.cell((x, y)));
                            }
                        }
                    }
                }
            }

            app.path_copy_flash = Some((
                PathKind::Absolute,
                true,
                Instant::now() - Duration::from_millis(FLASH_DURATION_MS + 1),
            ));
            assert_eq!(draw(&mut app, width), idle, "expired status must clear");
        }
    }
}
