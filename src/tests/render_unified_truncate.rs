use super::common::render_helpers::*;
use crate::render::FrameDecorations;

#[test]
fn unified_top_and_hunk_headers_unchanged() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut top = vec![Line::from(Span::styled("┌─".to_string(), Style::default()))];
    crate::render::close_frame_right(
        &mut top,
        40,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let top_text = line_text(&top[0]);
    assert!(
        !top_text.contains('┬'),
        "unified top row must not contain `┬`, got {top_text:?}"
    );
    let mut mid = vec![Line::from(vec![
        Span::styled("├─ ".to_string(), Style::default()),
        Span::styled("around line 4".to_string(), Style::default()),
    ])];
    crate::render::close_frame_right(
        &mut mid,
        40,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let mid_text = line_text(&mid[0]);
    assert!(
        !mid_text.contains('┼'),
        "unified hunk header row must not contain `┼`, got {mid_text:?}"
    );
}

#[test]
fn unified_body_row_truncated_with_ellipsis() {
    use crate::theme::app_theme;
    let width: u16 = 40;
    let theme = app_theme();
    let add_bg = theme.diff.add_bg;
    let long = "A".repeat(200);
    let cells = render_unified_row_cells('+', add_bg, &long, width);
    assert_eq!(
        cells.len() as u16,
        width,
        "row must fill exactly `width` cells"
    );

    let last = &cells[(width as usize) - 1];
    assert_eq!(last.0, "│", "column width-1 must be the right frame `│`");

    let ellipsis = &cells[(width as usize) - 2];
    assert_eq!(
        ellipsis.0, "…",
        "column width-2 must be the `…` truncation marker"
    );

    for (x, cell) in cells.iter().enumerate().take(width as usize - 1).skip(1) {
        assert_eq!(
            cell.1, add_bg,
            "cell at col {x} must carry add_bg through the `…` marker, got {:?} (sym {:?})",
            cell.1, cell.0
        );
    }
}

#[test]
fn unified_del_row_truncated_with_ellipsis() {
    use crate::theme::app_theme;
    let width: u16 = 40;
    let theme = app_theme();
    let del_bg = theme.diff.del_bg;
    let long = "D".repeat(200);
    let cells = render_unified_row_cells('-', del_bg, &long, width);
    assert_eq!(cells.len() as u16, width);
    assert_eq!(cells[(width as usize) - 1].0, "│");
    assert_eq!(cells[(width as usize) - 2].0, "…");
    for (x, cell) in cells.iter().enumerate().take(width as usize - 1).skip(1) {
        assert_eq!(
            cell.1, del_bg,
            "cell at col {x} must carry del_bg through the `…` marker, got {:?} (sym {:?})",
            cell.1, cell.0
        );
    }
}

#[test]
fn unified_short_content_not_truncated() {
    use ratatui::style::{Color, Style};
    use ratatui::text::Span;
    let theme = diff_theme_for_test();
    let bg = Color::Reset;
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled("│".to_string(), Style::default().fg(theme.frame_fg)),
        Span::styled(" ".to_string(), Style::default()),
        Span::styled(" ".to_string(), Style::default()),
        Span::styled("short".to_string(), Style::default()),
    ];
    let before: String = spans.iter().map(|s| s.content.clone()).collect();
    crate::render::truncate_unified_body_row(&mut spans, 40, Some(bg));
    let after: String = spans.iter().map(|s| s.content.clone()).collect();
    assert_eq!(
        before, after,
        "short unified row must not be modified — no `…` added"
    );
    assert!(
        !after.contains('…'),
        "short unified row must not carry a `…` marker, got {after:?}"
    );
}
