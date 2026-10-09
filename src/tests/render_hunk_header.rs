use super::common::render_helpers::*;
use super::test_assets;
use crate::render::FrameDecorations;

#[test]
fn hunk_header_uses_short_counts_added_only() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -3,1 +4,3 @@
 keep
+one
+two
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let text = built
        .lines
        .iter()
        .find_map(|l| {
            let t = line_text(l);
            t.starts_with("├─ around line ").then_some(t)
        })
        .expect("short-count hunk header present");
    assert_eq!(text, "├─ around line 4 · +2 −0");
}

#[test]
fn hunk_header_uses_short_counts_mixed() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -3,2 +4,2 @@
 keep
-old
+new
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let text = built
        .lines
        .iter()
        .find_map(|l| {
            let t = line_text(l);
            t.starts_with("├─ around line ").then_some(t)
        })
        .expect("short-count hunk header present");
    assert_eq!(text, "├─ around line 4 · +1 −1");
}

#[test]
fn hunk_header_omits_counts_when_both_zero() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -3,2 +3,2 @@
 keep
 also_keep
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let text = built
        .lines
        .iter()
        .find_map(|l| {
            let t = line_text(l);
            t.starts_with("├─ around line ").then_some(t)
        })
        .expect("zero-count hunk header present");
    assert_eq!(text, "├─ around line 3");
    assert!(!text.contains('·'), "no separator when both counts are 0");
    assert!(!text.contains('+'), "no `+` when both counts are 0");
    assert!(!text.contains('−'), "no `−` when both counts are 0");
}

#[test]
fn hunk_header_uses_tee_prefix() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -3,1 +4,2 @@
 keep
+one
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let hdr = built
        .lines
        .iter()
        .find(|l| line_text(l).starts_with("├─ around line "))
        .expect("hunk header with tee prefix present");
    let first = hdr
        .spans
        .first()
        .expect("hunk header has at least one span");
    assert_eq!(
        first.content, "├─ ",
        "hunk header must start with `├─ ` as first span, got {:?}",
        first.content
    );
}

#[test]
fn hunk_header_row_uses_pipe_not_tee() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(vec![
        Span::styled("├─ ".to_string(), Style::default()),
        Span::styled("around line 4 · 1 line added".to_string(), Style::default()),
    ])];
    crate::render::close_frame_right(
        &mut lines,
        80,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);
    assert!(
        text.starts_with("├─"),
        "row must start with `├─`, got {text:?}"
    );
    assert_eq!(last_char(&lines[0]), Some('│'));
    assert!(
        !text.contains('┤'),
        "hunk header row must not contain `┤`, got {text:?}"
    );
    let interior: String = text
        .chars()
        .skip("├─ around line 4 · 1 line added".chars().count())
        .take_while(|c| *c != '│')
        .collect();
    assert!(
        !interior.is_empty() && interior.chars().all(|c| c == ' '),
        "├─ row interior fill must be spaces, got {interior:?}"
    );
    assert_eq!(line_width_chars(&lines[0]), 80);
}

#[test]
fn hunk_header_prefix_uses_frame_fg_and_dot_separator() {
    use ratatui::style::Modifier;
    let diff_theme = diff_theme_for_test();
    let ui_theme = crate::theme::app_theme().ui;
    let hdr = build_mixed_hunk_header_line();
    let prefix = &hdr.spans[0];
    assert_eq!(prefix.content, "├─ ");
    assert_eq!(prefix.style.fg, Some(diff_theme.frame_fg));
    assert!(!prefix.style.add_modifier.contains(Modifier::BOLD));

    let md_theme = crate::theme::app_theme().markdown;
    let _ = ui_theme;
    let sep = &hdr.spans[3];
    assert_eq!(sep.content, " · ");
    assert_eq!(sep.style.fg, Some(md_theme.mermaid_block_fg));
    assert!(!sep.style.add_modifier.contains(Modifier::BOLD));
    let gap = &hdr.spans[5];
    assert_eq!(gap.content, " ");
    assert_eq!(gap.style.fg, Some(diff_theme.frame_fg));
    assert!(!gap.style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn hunk_header_line_word_uses_hunk_header_fg() {
    let hdr = build_mixed_hunk_header_line();
    let diff_theme = diff_theme_for_test();
    let line_num = &hdr.spans[2];
    assert_eq!(line_num.content, "line 4");
    assert_eq!(line_num.style.fg, Some(diff_theme.hunk_header_fg));
}

#[test]
fn hunk_header_around_word_uses_hunk_header_fg() {
    let hdr = build_mixed_hunk_header_line();
    let diff_theme = diff_theme_for_test();
    let around = &hdr.spans[1];
    assert_eq!(around.content, "around ");
    assert_eq!(around.style.fg, Some(diff_theme.hunk_header_fg));
}

#[test]
fn hunk_header_add_count_uses_add_fg() {
    use ratatui::style::Modifier;
    let diff_theme = diff_theme_for_test();
    let hdr = build_mixed_hunk_header_line();
    let add = &hdr.spans[4];
    assert_eq!(add.content, "+1");
    assert_eq!(add.style.fg, Some(diff_theme.add_fg));
    assert!(!add.style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn hunk_header_del_count_uses_del_fg() {
    use ratatui::style::Modifier;
    let diff_theme = diff_theme_for_test();
    let hdr = build_mixed_hunk_header_line();
    let del = &hdr.spans[6];
    assert_eq!(del.content, "−1");
    assert_eq!(del.style.fg, Some(diff_theme.del_fg));
    assert!(!del.style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn hunk_header_around_word_is_not_bold() {
    use ratatui::style::Modifier;
    let hdr = build_mixed_hunk_header_line();
    let around = &hdr.spans[1];
    assert_eq!(around.content, "around ");
    assert!(
        !around.style.add_modifier.contains(Modifier::BOLD),
        "`around ` span must NOT be BOLD"
    );
}

#[test]
fn hunk_header_line_word_is_not_bold() {
    use ratatui::style::Modifier;
    let hdr = build_mixed_hunk_header_line();
    let line_num = &hdr.spans[2];
    assert_eq!(line_num.content, "line 4");
    assert!(
        !line_num.style.add_modifier.contains(Modifier::BOLD),
        "`line NN` span must NOT be BOLD"
    );
}

#[test]
fn hunk_header_counts_are_not_bold() {
    use ratatui::style::Modifier;
    let hdr = build_mixed_hunk_header_line();
    let add = &hdr.spans[4];
    assert_eq!(add.content, "+1");
    assert!(
        !add.style.add_modifier.contains(Modifier::BOLD),
        "`+M` count span must NOT be BOLD"
    );
    let del = &hdr.spans[6];
    assert_eq!(del.content, "−1");
    assert!(
        !del.style.add_modifier.contains(Modifier::BOLD),
        "`−K` count span must NOT be BOLD"
    );
}
