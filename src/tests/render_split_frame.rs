use super::common::render_helpers::*;
use super::test_assets;
use crate::render::FrameDecorations;

#[test]
fn split_bottom_tee_aligns_with_middle_only() {
    let w: u16 = 200;
    let buffer = render_split_at_width(w, true);

    let mut bottom_row: Option<u16> = None;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer.cell((x, y)).unwrap().symbol() == "└" {
                bottom_row = Some(y);
                break;
            }
        }
        if bottom_row.is_some() {
            break;
        }
    }
    let bottom = bottom_row.expect("expected a `└` bottom row in split view");

    let mut tees: Vec<u16> = Vec::new();
    for x in 0..buffer.area.width {
        if buffer.cell((x, bottom)).unwrap().symbol() == "┴" {
            tees.push(x);
        }
    }

    let middle_col_expected = 1u16 + (w - 3) / 2;
    assert!(
        tees.contains(&middle_col_expected),
        "expected `┴` at middle col {middle_col_expected}, got tees {tees:?}"
    );

    assert!(
        tees.iter().any(|c| *c > middle_col_expected),
        "expected a `┴` beyond the middle (RIGHT gutter cap), got {tees:?}"
    );
}

#[test]
fn split_top_row_has_no_left_gutter_tee() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled(
        "┌─ x ".to_string(),
        Style::default(),
    ))];
    let status: Vec<Option<Vec<Span<'static>>>> = vec![Some(vec![Span::styled(
        "modified".to_string(),
        Style::default(),
    )])];

    crate::render::close_frame_right(
        &mut lines,
        80,
        &theme,
        &FrameDecorations {
            status_by_line: Some(&status),
            gutter_widths: Some((3, 3)),
            ..Default::default()
        },
    );
    let chars: Vec<char> = line_text(&lines[0]).chars().collect();
    assert_ne!(
        chars.get(8),
        Some(&'┬'),
        "split top row must not have `┬` at LEFT gutter col 8, got {:?}",
        chars.get(8)
    );
    assert_eq!(
        chars.get(8),
        Some(&'─'),
        "split top row LEFT gutter col 8 must be `─`, got {:?}",
        chars.get(8)
    );

    let mut lines2 = vec![Line::from(Span::styled(
        "┌─ x ".to_string(),
        Style::default(),
    ))];
    crate::render::close_frame_right(
        &mut lines2,
        80,
        &theme,
        &FrameDecorations {
            status_by_line: Some(&status),
            gutter_widths: Some((3, 3)),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let chars2: Vec<char> = line_text(&lines2[0]).chars().collect();
    assert_eq!(
        chars2.get(8),
        Some(&'┬'),
        "unified top row at LEFT gutter col 8 must be `┬`, got {:?}",
        chars2.get(8)
    );
}

#[test]
fn split_hunk_header_has_no_left_gutter_tee() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled(
        "├─ ".to_string(),
        Style::default(),
    ))];

    crate::render::close_frame_right(
        &mut lines,
        80,
        &theme,
        &FrameDecorations {
            gutter_widths: Some((3, 3)),
            ..Default::default()
        },
    );
    let chars: Vec<char> = line_text(&lines[0]).chars().collect();
    assert_ne!(
        chars.get(8),
        Some(&'┼'),
        "split hunk-header row must not have `┼` at LEFT gutter col 8, got {:?}",
        chars.get(8)
    );

    assert_eq!(
        chars.get(8),
        Some(&' '),
        "split hunk-header row LEFT gutter col 8 must be ` `, got {:?}",
        chars.get(8)
    );

    let mut lines2 = vec![Line::from(Span::styled(
        "├─ ".to_string(),
        Style::default(),
    ))];
    crate::render::close_frame_right(
        &mut lines2,
        80,
        &theme,
        &FrameDecorations {
            gutter_widths: Some((3, 3)),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let chars2: Vec<char> = line_text(&lines2[0]).chars().collect();
    assert_eq!(
        chars2.get(8),
        Some(&'┼'),
        "unified hunk-header row at LEFT gutter col 8 must be `┼`, got {:?}",
        chars2.get(8)
    );
}

#[test]
fn split_bottom_row_caps_left_gutter_with_tee() {
    let w: u16 = 200;
    let buffer = render_split_at_width(w, true);
    let gutter_col = left_gutter_col_split(&buffer).expect("body row has LEFT gutter │");

    let mut bottom_row: Option<u16> = None;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer.cell((x, y)).unwrap().symbol() == "└" {
                bottom_row = Some(y);
                break;
            }
        }
        if bottom_row.is_some() {
            break;
        }
    }
    let bottom = bottom_row.expect("expected a `└` bottom row in split view");
    let sym = buffer.cell((gutter_col, bottom)).unwrap().symbol();
    assert_eq!(
        sym, "┴",
        "split bottom row must have `┴` at LEFT gutter col {gutter_col}, got {sym:?}"
    );
}

#[test]
fn split_bottom_has_tees_at_middle_left_right_gutters() {
    let w: u16 = 200;
    let buffer = render_split_at_width(w, true);

    let mut bottom_row: Option<u16> = None;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer.cell((x, y)).unwrap().symbol() == "└" {
                bottom_row = Some(y);
                break;
            }
        }
        if bottom_row.is_some() {
            break;
        }
    }
    let bottom = bottom_row.expect("expected a `└` bottom row in split view");

    let middle_col = 1u16 + (w - 3) / 2;

    let left_gutter_col = left_gutter_col_split(&buffer).expect("body row has LEFT gutter │");

    let right_gutter_col = middle_col + (left_gutter_col - 1);
    let sym_mid = buffer.cell((middle_col, bottom)).unwrap().symbol();
    let sym_left = buffer.cell((left_gutter_col, bottom)).unwrap().symbol();
    let sym_right = buffer.cell((right_gutter_col, bottom)).unwrap().symbol();
    assert_eq!(
        sym_mid, "┴",
        "expected `┴` at middle col {middle_col} of bottom row, got {sym_mid:?}"
    );
    assert_eq!(
        sym_left, "┴",
        "expected `┴` at LEFT gutter col {left_gutter_col} of bottom row, got {sym_left:?}"
    );
    assert_eq!(
        sym_right, "┴",
        "expected `┴` at RIGHT gutter col {right_gutter_col} of bottom row, got {sym_right:?}"
    );
}

#[test]
fn split_top_still_has_only_middle_tee() {
    let w: u16 = 200;
    let buffer = render_split_at_width(w, true);

    let mut top_row: Option<u16> = None;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer.cell((x, y)).unwrap().symbol() == "┌" {
                top_row = Some(y);
                break;
            }
        }
        if top_row.is_some() {
            break;
        }
    }
    let top = top_row.expect("expected a `┌` top row in split view");
    let middle_col = 1u16 + (w - 3) / 2;
    let left_gutter_col = left_gutter_col_split(&buffer).expect("body row has LEFT gutter │");
    let right_gutter_col = middle_col + (left_gutter_col - 1);

    let sym_mid = buffer.cell((middle_col, top)).unwrap().symbol();
    let sym_left = buffer.cell((left_gutter_col, top)).unwrap().symbol();
    let sym_right = buffer.cell((right_gutter_col, top)).unwrap().symbol();
    assert_eq!(
        sym_mid, "┬",
        "expected `┬` at middle col {middle_col} of top row, got {sym_mid:?}"
    );
    assert_ne!(
        sym_left, "┬",
        "top row must NOT have `┬` at LEFT gutter col {left_gutter_col}"
    );
    assert_ne!(
        sym_right, "┬",
        "top row must NOT have `┬` at RIGHT gutter col {right_gutter_col}"
    );

    let mut count = 0usize;
    for x in 0..buffer.area.width {
        if buffer.cell((x, top)).unwrap().symbol() == "┬" {
            count += 1;
        }
    }
    assert_eq!(
        count, 1,
        "expected exactly 1 `┬` (middle only) on the split top row, got {count}"
    );
}

#[test]
fn split_no_cross_char_anywhere() {
    for line_numbers in [false, true] {
        let w: u16 = 120;
        let buffer = render_split_at_width(w, line_numbers);
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let sym = buffer.cell((x, y)).unwrap().symbol();
                assert_ne!(
                    sym, "┼",
                    "unexpected `┼` at ({x},{y}) with line_numbers={line_numbers}"
                );
            }
        }
    }
}

#[test]
fn split_hunk_header_middle_is_dotted() {
    let w: u16 = 100;
    let buffer = render_split_at_width(w, false);

    let mut header_row: Option<u16> = None;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer.cell((x, y)).unwrap().symbol() == "├" {
                header_row = Some(y);
                break;
            }
        }
        if header_row.is_some() {
            break;
        }
    }
    let row = header_row.expect("expected a `├` hunk header row in split view");
    let middle_col = 1u16 + (w - 3) / 2;
    let cell = buffer.cell((middle_col, row)).unwrap();
    assert_eq!(
        cell.symbol(),
        "│",
        "hunk header row must carry solid `│` at middle col, got {:?}",
        cell.symbol()
    );
}

#[test]
fn split_view_sticky_file_top_pins_when_scrolled() {
    use crate::app::{App, DiffSpec};
    use ratatui::{backend::TestBackend, Terminal};

    let mut src = String::from("diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,20 +1,20 @@\n");
    for i in 0..20 {
        src.push_str(&format!(" line {i}\n"));
    }
    let files = crate::diff::parse_unified_diff(&src);
    let mut app = App::new(
        vec![],
        vec![],
        "stdin".to_string(),
        false,
        false,
        None,
        None,
    );
    let (ss, theme) = test_assets();
    app.set_source_for_test(src.clone());
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    app.build_split_now(&ss, &theme);
    app.diff_mut().unwrap().layout = crate::diff::DiffLayout::Split;

    app.scroll_to(5);
    let mut terminal = Terminal::new(TestBackend::new(200, 12)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let mut row0 = String::new();
    for x in 0..buffer.area.width {
        row0.push_str(buffer.cell((x, 0)).unwrap().symbol());
    }
    assert!(
        row0.contains('┌'),
        "expected sticky `┌` on row 0 in split view when scrolled, got: {row0:?}"
    );
}

#[test]
fn split_top_has_down_tee_at_middle_only() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled("┌─".to_string(), Style::default()))];
    let middle_col = 20usize;
    let right_gutter_col = 34usize;
    let tees = [middle_col];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            extra_top_tees: &tees,
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);
    assert_eq!(last_char(&lines[0]), Some('┐'));
    assert_eq!(line_width_chars(&lines[0]), 40);
    let chars: Vec<char> = text.chars().collect();
    assert_eq!(
        chars.get(middle_col),
        Some(&'┬'),
        "expected `┬` at middle_col {middle_col} of top row, got {text:?}"
    );
    assert_eq!(
        chars.get(right_gutter_col),
        Some(&'─'),
        "expected plain `─` at right_gutter_col {right_gutter_col} of top row (no connector), got {text:?}"
    );
    assert_eq!(
        text.chars().filter(|c| *c == '┬').count(),
        1,
        "exactly one `┬` char (middle only) must appear on the split top row, got {text:?}"
    );
}

#[test]
fn split_hunk_header_middle_is_dotted_right_gutter_is_plain() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(vec![
        Span::styled("├─ ".to_string(), Style::default()),
        Span::styled("around line 4".to_string(), Style::default()),
    ])];
    let middle_col = 20usize;
    let right_gutter_col = 34usize;
    let cross_tees: [usize; 0] = [];
    let dotted_tees = [middle_col];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            extra_middle_tees: &cross_tees,
            extra_middle_dotted_tees: &dotted_tees,
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);
    assert_eq!(last_char(&lines[0]), Some('│'));
    assert_eq!(line_width_chars(&lines[0]), 40);
    let chars: Vec<char> = text.chars().collect();
    assert_eq!(
        chars.get(middle_col),
        Some(&'│'),
        "expected `│` at middle_col {middle_col} of hunk row, got {text:?}"
    );
    assert_ne!(
        chars.get(right_gutter_col),
        Some(&'┼'),
        "expected NOT `┼` at right_gutter_col {right_gutter_col} of hunk row (no connector), got {text:?}"
    );
    assert_eq!(
        text.chars().filter(|c| *c == '┼').count(),
        0,
        "no `┼` char must appear on the split hunk row, got {text:?}"
    );

    assert_eq!(
        text.chars().filter(|c| *c == '│').count(),
        2,
        "expected 2 `│` chars on the split hunk row (middle + right border), got {text:?}"
    );
}

#[test]
fn split_bottom_tee_at_middle_only() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled("└─".to_string(), Style::default()))];
    let middle_col = 20usize;
    let right_gutter_col = 34usize;
    let tees = [middle_col];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            extra_bottom_tees: &tees,
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);
    assert_eq!(last_char(&lines[0]), Some('┘'));
    assert_eq!(line_width_chars(&lines[0]), 40);
    let chars: Vec<char> = text.chars().collect();
    assert_eq!(chars.get(middle_col), Some(&'┴'));
    assert_eq!(
        chars.get(right_gutter_col),
        Some(&'─'),
        "expected plain `─` at right_gutter_col {right_gutter_col} of bottom row (no connector), got {text:?}"
    );
    assert_eq!(
        text.chars().filter(|c| *c == '┴').count(),
        1,
        "exactly one `┴` char (middle only) must appear on the split bottom row, got {text:?}"
    );
}

#[test]
fn split_no_solid_pipe_in_middle_column() {
    let w: u16 = 100;
    let middle_col = 1u16 + (w - 3) / 2;
    for line_numbers in [false, true] {
        let buffer = render_split_at_width(w, line_numbers);
        for y in 0..buffer.area.height {
            let sym = buffer.cell((middle_col, y)).unwrap().symbol();
            assert_ne!(
                sym, "┊",
                "row {y} (line_numbers={line_numbers}) has dotted `┊` at middle col {middle_col}"
            );
            assert_ne!(
                sym, "┼",
                "row {y} (line_numbers={line_numbers}) has cross `┼` at middle col {middle_col}"
            );
        }
    }
}
