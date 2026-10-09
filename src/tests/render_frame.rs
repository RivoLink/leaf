use super::common::render_helpers::*;
use super::test_assets;
use crate::render::FrameDecorations;

#[test]
fn body_line_no_trailing_padding() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,1 +1,2 @@
 keep
+new
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let add_idx = built
        .line_new_num
        .iter()
        .zip(built.line_old_num.iter())
        .position(|(n, o)| *n > 0 && *o == 0)
        .expect("add body row present");
    let add_line = &built.lines[add_idx];
    let last = add_line.spans.last().unwrap();
    assert!(
        last.content.len() < 300,
        "expected no ~300-space trailing pad, got last span len {}",
        last.content.len()
    );
    assert!(
        !last.content.chars().all(|c| c == ' ') || last.content.is_empty(),
        "last span should not be pure whitespace (that was the old pad), got {:?}",
        last.content
    );
}

#[test]
fn body_lines_prefixed_with_left_border() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,2 +1,3 @@
 keep
-old
+new
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let body_line_indices: Vec<usize> = built
        .line_new_num
        .iter()
        .zip(built.line_old_num.iter())
        .enumerate()
        .filter_map(|(i, (n, o))| (*n > 0 || *o > 0).then_some(i))
        .collect();
    assert!(
        body_line_indices.len() >= 3,
        "expected ≥3 body lines (ctx + add + del), got {}",
        body_line_indices.len()
    );
    for i in body_line_indices {
        let first = built.lines[i].spans.first().expect("border span present");
        assert_eq!(
            first.content, "│",
            "body line must start with bare `│` border span (no trailing space), got {:?}",
            first.content
        );
    }
}

#[test]
fn body_line_no_gutter_sigil_hugs_border() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,2 +1,3 @@
 keep
-old
+new
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let find_body = |pred: &dyn Fn(u32, u32) -> bool| -> &ratatui::text::Line<'static> {
        for (i, (o, n)) in built
            .line_old_num
            .iter()
            .zip(built.line_new_num.iter())
            .enumerate()
        {
            if pred(*o, *n) {
                return &built.lines[i];
            }
        }
        panic!("no matching body row found");
    };

    let ctx = find_body(&|o, n| o > 0 && n > 0);
    assert_eq!(
        ctx.spans[0].content, "│",
        "context: first span must be bare `│`"
    );
    assert_eq!(
        ctx.spans[1].content.chars().count(),
        1,
        "context: second span (sigil) must be a single char"
    );
    assert_eq!(ctx.spans[1].content, " ", "context: sigil must be a space");
    assert_eq!(
        ctx.spans[2].content, " ",
        "context: third span must be the separator space"
    );

    let add = find_body(&|o, n| o == 0 && n > 0);
    assert_eq!(
        add.spans[0].content, "│",
        "add: first span must be bare `│`"
    );
    assert_eq!(add.spans[1].content, "+", "add: sigil must be `+`");
    assert_eq!(
        add.spans[2].content, " ",
        "add: third span must be the separator space"
    );

    assert_eq!(
        add.spans[0].style.bg, None,
        "add: left border `│` must NOT carry a bg"
    );
    assert!(
        add.spans[1].style.bg.is_some(),
        "add: sigil must carry a bg (add_bg)"
    );

    let del = find_body(&|o, n| o > 0 && n == 0);
    assert_eq!(
        del.spans[0].content, "│",
        "del: first span must be bare `│`"
    );
    assert_eq!(del.spans[1].content, "-", "del: sigil must be `-`");
    assert_eq!(
        del.spans[2].content, " ",
        "del: third span must be the separator space"
    );
    assert_eq!(
        del.spans[0].style.bg, None,
        "del: left border `│` must NOT carry a bg"
    );
    assert!(
        del.spans[1].style.bg.is_some(),
        "del: sigil must carry a bg (del_bg)"
    );
}

#[test]
fn close_frame_right_adds_top_corner() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled(
        "┌─ foo.rs ".to_string(),
        Style::default(),
    ))];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);
    assert_eq!(last_char(&lines[0]), Some('┐'));
    assert!(
        text.contains('─'),
        "top-frame fill should use ─, got {text:?}"
    );
    assert_eq!(line_width_chars(&lines[0]), 40);
}

#[test]
fn close_frame_right_adds_bottom_corner() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled("└─".to_string(), Style::default()))];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);
    assert_eq!(last_char(&lines[0]), Some('┘'));
    assert!(
        text.chars().filter(|c| *c == '─').count() >= 3,
        "bottom-frame fill should use ─ across the row, got {text:?}"
    );
    assert_eq!(line_width_chars(&lines[0]), 40);
}

#[test]
fn close_frame_right_hunk_row_uses_pipe() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(vec![
        Span::styled("├─ ".to_string(), Style::default()),
        Span::styled("around line 16".to_string(), Style::default()),
    ])];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );

    assert_eq!(last_char(&lines[0]), Some('│'));

    let text = line_text(&lines[0]);
    let interior: String = text
        .chars()
        .skip("├─ around line 16".chars().count())
        .take_while(|c| *c != '│')
        .collect();
    assert!(
        !interior.is_empty() && interior.chars().all(|c| c == ' '),
        "├─ row interior fill must be spaces, got {interior:?}"
    );
    assert_eq!(line_width_chars(&lines[0]), 40);
}

#[test]
fn close_frame_right_adds_pipe() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(vec![
        Span::styled("│ ".to_string(), Style::default()),
        Span::styled("+ let x = 1;".to_string(), Style::default()),
    ])];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    assert_eq!(last_char(&lines[0]), Some('│'));
    assert_eq!(line_width_chars(&lines[0]), 40);
}

#[test]
fn close_frame_right_skips_non_bordered_lines() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled("".to_string(), Style::default()))];
    let before = line_width_chars(&lines[0]);
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    assert_eq!(line_width_chars(&lines[0]), before);

    let mut lines = vec![Line::from(Span::styled(
        "hello".to_string(),
        Style::default(),
    ))];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    assert_eq!(line_text(&lines[0]), "hello");
}

#[test]
fn close_frame_right_handles_content_narrower_than_width() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled(
        "│ short".to_string(),
        Style::default(),
    ))];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    assert_eq!(last_char(&lines[0]), Some('│'));
}

#[test]
fn close_frame_right_handles_content_wider_than_width() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let overflow = format!("│ {}", "x".repeat(50));
    let mut lines = vec![Line::from(Span::styled(overflow.clone(), Style::default()))];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );

    assert_eq!(line_text(&lines[0]), overflow);
}

#[test]
fn frame_uses_frame_fg() {
    use ratatui::style::Modifier;
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x.rs b/x.rs
--- a/x.rs
+++ b/x.rs
@@ -1,1 +1,2 @@
 keep
+new
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let mut lines = built.lines.clone();
    let status = built.file_header_status.clone();
    let diff_theme = diff_theme_for_test();
    crate::render::close_frame_right(
        &mut lines,
        80,
        &diff_theme,
        &FrameDecorations {
            status_by_line: Some(&status),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );

    let frame_chars = ['┌', '┐', '└', '┘', '│', '├', '─'];
    let mut seen_any = false;
    for line in &lines {
        for span in &line.spans {
            let only_frame =
                !span.content.is_empty() && span.content.chars().all(|c| frame_chars.contains(&c));
            if !only_frame {
                continue;
            }
            seen_any = true;
            assert_eq!(
                span.style.fg,
                Some(diff_theme.frame_fg),
                "expected frame_fg for pure-frame span {:?}",
                span.content
            );
            assert!(
                !span.style.add_modifier.contains(Modifier::BOLD),
                "frame span {:?} must not carry BOLD",
                span.content
            );
        }
    }
    assert!(seen_any, "expected to see at least one frame-only span");
}

#[test]
fn body_line_first_span_is_bare_left_pipe() {
    let (diff_theme, lines, state) = build_gutter_lines_for_test();
    let spans = find_gutter_line_for(&lines, &state, |_o, _n| true);
    let first = spans.first().expect("body row has spans");
    assert_eq!(
        first.content, "│",
        "body row's first span must be exactly `│` (no trailing space), got {:?}",
        first.content
    );
    assert_eq!(
        first.style.fg,
        Some(diff_theme.frame_fg),
        "left border `│` must carry frame_fg"
    );
}

#[test]
fn frame_line_first_span_is_bare_left_pipe_unchanged() {
    let (diff_theme, lines, _state) = build_gutter_lines_for_test();

    let top = lines
        .iter()
        .find(|l| {
            l.spans
                .first()
                .map(|s| s.content.starts_with('┌'))
                .unwrap_or(false)
        })
        .expect("file top frame present");

    let blank = lines
        .iter()
        .find(|l| l.spans.len() == 1 && l.spans[0].content == "│")
        .expect("bordered blank spacer present");
    assert_eq!(
        blank.spans[0].content, "│",
        "blank interior spacer must be a bare `│` (unchanged by gutter)"
    );
    assert_eq!(
        blank.spans[0].style.fg,
        Some(diff_theme.frame_fg),
        "blank interior spacer must be frame_fg"
    );

    assert!(
        top.spans[0].content.starts_with("┌─ "),
        "file top frame must still start with `┌─ ` (unchanged), got {:?}",
        top.spans[0].content
    );
}

#[test]
fn body_content_starts_with_sigil_after_right_pipe() {
    let diff_theme = diff_theme_for_test();
    let (_, lines, state) = build_gutter_lines_for_test();
    let spans = find_gutter_line_for(&lines, &state, |o, n| n > 0 && o == 0);

    let outer_border = &spans[0];
    let pipe_span = &spans[4];
    let sigil_span = &spans[5];
    let sep_span = &spans[6];
    assert_eq!(outer_border.content, "│");
    assert_eq!(pipe_span.content, "│");
    assert_eq!(sigil_span.content, "+");
    assert_eq!(
        sigil_span.content.chars().count(),
        1,
        "sigil after the right `│` must be exactly one char"
    );
    assert_eq!(sep_span.content, " ");
    assert_eq!(
        outer_border.style.bg, None,
        "the outer left `│` border must NOT carry add_bg (strip stops at the box border)"
    );
    assert_eq!(
        pipe_span.style.bg,
        Some(diff_theme.add_bg),
        "the inner gutter→content `│` carries add_bg so the strip is continuous"
    );
    assert!(
        sigil_span.style.bg.is_some(),
        "add-row sigil must carry a bg (add_bg), got {:?}",
        sigil_span.style.bg
    );
}

#[test]
fn body_sigil_char_is_single_char() {
    let (_diff_theme, lines, state) = build_gutter_lines_for_test();
    for pred in [
        Box::new(|o: u32, n: u32| n > 0 && o == 0) as Box<dyn Fn(u32, u32) -> bool>,
        Box::new(|o, n| o > 0 && n == 0),
        Box::new(|o, n| o > 0 && n > 0),
    ] {
        let spans = find_gutter_line_for(&lines, &state, pred);
        let sigil = &spans[5];
        assert_eq!(
            sigil.content.chars().count(),
            1,
            "sigil span must be a single char, got {:?}",
            sigil.content
        );
    }
}

#[test]
fn body_sigil_uses_add_fg_add_bg_for_add_row() {
    let (diff_theme, lines, state) = build_gutter_lines_for_test();
    let spans = find_gutter_line_for(&lines, &state, |o, n| n > 0 && o == 0);
    let sigil = &spans[5];
    assert_eq!(sigil.content, "+");
    assert_eq!(sigil.style.fg, Some(diff_theme.add_fg));
    assert_eq!(sigil.style.bg, Some(diff_theme.add_bg));
}

#[test]
fn body_sigil_uses_del_fg_del_bg_for_del_row() {
    let (diff_theme, lines, state) = build_gutter_lines_for_test();
    let spans = find_gutter_line_for(&lines, &state, |o, n| o > 0 && n == 0);
    let sigil = &spans[5];
    assert_eq!(sigil.content, "-");
    assert_eq!(sigil.style.fg, Some(diff_theme.del_fg));
    assert_eq!(sigil.style.bg, Some(diff_theme.del_bg));
}

#[test]
fn body_sigil_is_space_for_context_row() {
    let (_diff_theme, lines, state) = build_gutter_lines_for_test();
    let spans = find_gutter_line_for(&lines, &state, |o, n| o > 0 && n > 0);
    let sigil = &spans[5];
    assert_eq!(sigil.content, " ");
    assert_eq!(sigil.style.fg, None);
    assert_eq!(sigil.style.bg, None);
}
