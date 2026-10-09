use super::common::render_helpers::*;
use super::test_assets;
use crate::render::FrameDecorations;

#[test]
fn build_diff_lines_applies_syntect_for_rust_extension() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,1 +1,1 @@
-let x: i32 = 1;
+let x: i32 = 2;
";
    let files = crate::diff::parse_unified_diff(src);
    assert!(!files.is_empty());
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let add_idx = built
        .line_new_num
        .iter()
        .zip(built.line_old_num.iter())
        .position(|(n, o)| *n > 0 && *o == 0)
        .expect("add body row present");
    let add_line = &built.lines[add_idx];

    assert!(
        add_line.spans.len() > 2,
        "expected syntect tokenization to produce multiple code spans, got {} spans total: {:?}",
        add_line.spans.len(),
        add_line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
    );
}

#[test]
fn build_diff_lines_falls_back_when_extension_unknown() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/notes b/notes
--- a/notes
+++ b/notes
@@ -1,1 +1,1 @@
-old text
+new text
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    assert!(built
        .lines
        .iter()
        .any(|l| !l.spans.is_empty() && !l.spans[0].content.is_empty()));
}

#[test]
fn path_stays_filename_fg_bold() {
    use ratatui::style::Modifier;
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/foo.rs b/foo.rs
--- a/foo.rs
+++ b/foo.rs
@@ -1,1 +1,2 @@
 keep
+new
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let diff_theme = diff_theme_for_test();
    let top = built
        .lines
        .iter()
        .find(|l| line_text(l).starts_with("┌─ "))
        .expect("file top frame present");

    assert!(top.spans.len() >= 2, "expected split top-frame spans");
    let path_span = &top.spans[1];
    assert!(
        path_span.content.contains("foo.rs"),
        "second span should carry the path, got {:?}",
        path_span.content
    );
    assert_eq!(
        path_span.style.fg,
        Some(diff_theme.filename_fg),
        "path span must be filename_fg"
    );
    assert!(
        path_span.style.add_modifier.contains(Modifier::BOLD),
        "path span must carry BOLD"
    );
}

#[test]
fn status_cluster_uses_hunk_header_fg_no_bold() {
    use ratatui::style::Modifier;
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,1 +1,2 @@
 keep
+one
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let diff_theme = diff_theme_for_test();
    let idx = built
        .lines
        .iter()
        .position(|l| line_text(l).starts_with("┌─ "))
        .expect("file top frame present");
    let status_spans = built.file_header_status[idx]
        .as_ref()
        .expect("status cluster present for file-top row");
    let label = &status_spans[0];
    assert_eq!(label.content, "modified");
    assert_eq!(
        label.style.fg,
        Some(diff_theme.hunk_header_fg),
        "status label must use hunk_header_fg, got {:?}",
        label.style.fg
    );
    assert!(
        !label.style.add_modifier.contains(Modifier::BOLD),
        "status label must not be BOLD, got {:?}",
        label.style.add_modifier
    );
}

#[test]
fn status_cluster_add_count_uses_add_fg() {
    use ratatui::style::Modifier;
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,1 +1,3 @@
 keep
+one
+two
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let diff_theme = diff_theme_for_test();
    let idx = built
        .lines
        .iter()
        .position(|l| line_text(l).starts_with("┌─ "))
        .expect("file top frame present");
    let status_spans = built.file_header_status[idx]
        .as_ref()
        .expect("status cluster present for file-top row");
    let add = status_spans
        .iter()
        .find(|s| s.content == "+2")
        .expect("+2 span present in status cluster");
    assert_eq!(
        add.style.fg,
        Some(diff_theme.add_fg),
        "+M span must be add_fg, got {:?}",
        add.style.fg
    );
    assert!(
        !add.style.add_modifier.contains(Modifier::BOLD),
        "+M span must not be BOLD, got {:?}",
        add.style.add_modifier
    );
}

#[test]
fn status_cluster_del_count_uses_del_fg() {
    use ratatui::style::Modifier;
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,2 +1,1 @@
 keep
-drop
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let diff_theme = diff_theme_for_test();
    let idx = built
        .lines
        .iter()
        .position(|l| line_text(l).starts_with("┌─ "))
        .expect("file top frame present");
    let status_spans = built.file_header_status[idx]
        .as_ref()
        .expect("status cluster present for file-top row");
    let del = status_spans
        .iter()
        .find(|s| s.content == "−1")
        .expect("−1 span present in status cluster");
    assert_eq!(
        del.style.fg,
        Some(diff_theme.del_fg),
        "−K span must be del_fg, got {:?}",
        del.style.fg
    );
    assert!(
        !del.style.add_modifier.contains(Modifier::BOLD),
        "−K span must not be BOLD, got {:?}",
        del.style.add_modifier
    );
}

#[test]
fn leading_blank_context_lines_are_trimmed() {
    let (ss, theme) = test_assets();

    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,2 +1,3 @@\n\n\n+content\n";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let header_idx = built
        .lines
        .iter()
        .position(|l| line_text(l).starts_with("├─ "))
        .expect("hunk header present");

    assert_eq!(
        line_text(&built.lines[header_idx + 1]),
        "│",
        "expected the built-in `│` spacer directly after the hunk header"
    );

    assert_eq!(built.line_old_num[header_idx + 2], 0);
    assert_eq!(
        built.line_new_num[header_idx + 2],
        3,
        "Add new-num must reflect the two skipped leading blank contexts"
    );

    let gap_blank_body_rows = (header_idx + 2..header_idx + 2)
        .filter(|i| {
            let t = line_text(&built.lines[*i]);
            t == "│  " || t == "│ "
        })
        .count();
    assert_eq!(
        gap_blank_body_rows, 0,
        "no blank body rows must survive the trim in the gap"
    );
}

#[test]
fn middle_blank_context_lines_preserved() {
    let (ss, theme) = test_assets();
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,3 +1,4 @@\n keep\n\n+added\n other\n";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let header_idx = built
        .lines
        .iter()
        .position(|l| line_text(l).starts_with("├─ "))
        .expect("hunk header present");

    assert_eq!(line_text(&built.lines[header_idx + 1]), "│");
    assert!(line_text(&built.lines[header_idx + 2]).contains("keep"));

    let middle = &built.lines[header_idx + 3];
    assert_eq!(
        line_text(middle),
        "│  ",
        "middle blank context row must be preserved"
    );
    assert!(
        built.line_old_num[header_idx + 3] > 0 && built.line_new_num[header_idx + 3] > 0,
        "middle blank context row must carry paired context gutter nums"
    );
    assert!(line_text(&built.lines[header_idx + 4]).contains("added"));
}

#[test]
fn trailing_blank_context_lines_are_trimmed() {
    let (ss, theme) = test_assets();

    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,2 +1,3 @@\n+added\n\n\n";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let header_idx = built
        .lines
        .iter()
        .position(|l| line_text(l).starts_with("├─ "))
        .expect("hunk header present");

    assert_eq!(line_text(&built.lines[header_idx + 1]), "│");
    assert!(
        line_text(&built.lines[header_idx + 2]).contains("added"),
        "Add row must be emitted"
    );

    for i in (header_idx + 3)..built.lines.len() {
        assert_ne!(
            line_text(&built.lines[i]),
            "│  ",
            "no trailing blank context body rows must survive the trim"
        );
    }
}

#[test]
fn both_leading_and_trailing_blanks_trimmed() {
    let (ss, theme) = test_assets();
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,3 +1,4 @@\n\n+added\n\n";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let header_idx = built
        .lines
        .iter()
        .position(|l| line_text(l).starts_with("├─ "))
        .expect("hunk header present");

    assert_eq!(line_text(&built.lines[header_idx + 1]), "│");
    assert!(
        line_text(&built.lines[header_idx + 2]).contains("added"),
        "content Add row must be emitted"
    );

    for i in (header_idx + 3)..built.lines.len() {
        assert_ne!(
            line_text(&built.lines[i]),
            "│  ",
            "no blank context body rows must survive when both trims apply"
        );
    }
}

#[test]
fn add_row_bg_extends_to_right_border() {
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(vec![
        Span::styled(
            "│".to_string(),
            ratatui::style::Style::default().fg(theme.frame_fg),
        ),
        Span::styled(
            "+".to_string(),
            ratatui::style::Style::default()
                .fg(theme.add_fg)
                .bg(theme.add_bg),
        ),
        Span::styled(
            " content".to_string(),
            ratatui::style::Style::default().bg(theme.add_bg),
        ),
    ])];

    let old_nums: Vec<u32> = vec![0];
    let new_nums: Vec<u32> = vec![1];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            line_nums: Some((&old_nums, &new_nums)),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    assert_eq!(last_char(&lines[0]), Some('│'));
    assert_eq!(line_width_chars(&lines[0]), 40);

    let n = lines[0].spans.len();
    assert!(n >= 2, "expected at least pad + right border spans");
    let pad_span = &lines[0].spans[n - 2];
    let right_span = &lines[0].spans[n - 1];
    assert_eq!(pad_span.style.bg, Some(theme.add_bg));
    assert!(pad_span.content.chars().all(|c| c == ' '));
    assert_eq!(right_span.content, "│");
    assert_eq!(right_span.style.bg, None);
}

#[test]
fn del_row_bg_extends_to_right_border() {
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(vec![
        Span::styled(
            "│".to_string(),
            ratatui::style::Style::default().fg(theme.frame_fg),
        ),
        Span::styled(
            "-".to_string(),
            ratatui::style::Style::default()
                .fg(theme.del_fg)
                .bg(theme.del_bg),
        ),
        Span::styled(
            " gone".to_string(),
            ratatui::style::Style::default().bg(theme.del_bg),
        ),
    ])];

    let old_nums: Vec<u32> = vec![1];
    let new_nums: Vec<u32> = vec![0];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            line_nums: Some((&old_nums, &new_nums)),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    assert_eq!(last_char(&lines[0]), Some('│'));
    assert_eq!(line_width_chars(&lines[0]), 40);
    let n = lines[0].spans.len();
    let pad_span = &lines[0].spans[n - 2];
    let right_span = &lines[0].spans[n - 1];
    assert_eq!(pad_span.style.bg, Some(theme.del_bg));
    assert!(pad_span.content.chars().all(|c| c == ' '));
    assert_eq!(right_span.content, "│");
    assert_eq!(right_span.style.bg, None);
}

#[test]
fn context_row_padding_uncolored() {
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(vec![
        Span::styled(
            "│".to_string(),
            ratatui::style::Style::default().fg(theme.frame_fg),
        ),
        Span::styled(" ".to_string(), ratatui::style::Style::default()),
        Span::styled(
            " ctx".to_string(),
            ratatui::style::Style::default().fg(theme.context_fg),
        ),
    ])];
    let old_nums: Vec<u32> = vec![1];
    let new_nums: Vec<u32> = vec![1];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            line_nums: Some((&old_nums, &new_nums)),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    assert_eq!(last_char(&lines[0]), Some('│'));
    assert_eq!(line_width_chars(&lines[0]), 40);

    for span in &lines[0].spans {
        assert_eq!(
            span.style.bg, None,
            "context row spans must not carry any bg, got {span:?}"
        );
    }
}

#[test]
fn intra_word_boundary_respected_on_rename() {
    let (ss, theme) = test_assets();
    let diff_theme = crate::theme::app_theme().diff;
    let src = "\
diff --git a/notes b/notes
--- a/notes
+++ b/notes
@@ -1,1 +1,1 @@
-let count_a = 5;
+let counter = 5;
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let del_idx = built
        .line_old_num
        .iter()
        .zip(built.line_new_num.iter())
        .position(|(o, n)| *o > 0 && *n == 0)
        .expect("del row present");
    let add_idx = built
        .line_new_num
        .iter()
        .zip(built.line_old_num.iter())
        .position(|(n, o)| *n > 0 && *o == 0)
        .expect("add row present");
    let del = body_content_char_bgs(&built.lines[del_idx]);
    let add = body_content_char_bgs(&built.lines[add_idx]);
    let del_text: String = del.iter().map(|(c, _)| *c).collect();
    let add_text: String = add.iter().map(|(c, _)| *c).collect();
    assert_eq!(del_text, "let count_a = 5;");
    assert_eq!(add_text, "let counter = 5;");

    for (i, (c, bg)) in del.iter().enumerate() {
        let expected = if (4..11).contains(&i) {
            diff_theme.intra_del
        } else {
            diff_theme.del_bg
        };
        assert_eq!(
            *bg,
            Some(expected),
            "del char {i} {c:?} bg mismatch: got {bg:?} expected {expected:?}"
        );
    }

    for (i, (c, bg)) in add.iter().enumerate() {
        let expected = if (4..11).contains(&i) {
            diff_theme.intra_add
        } else {
            diff_theme.add_bg
        };
        assert_eq!(
            *bg,
            Some(expected),
            "add char {i} {c:?} bg mismatch: got {bg:?} expected {expected:?}"
        );
    }
}

#[test]
fn intra_word_swap_produces_full_word_marks() {
    let (ss, theme) = test_assets();
    let diff_theme = crate::theme::app_theme().diff;
    let src = "\
diff --git a/notes b/notes
--- a/notes
+++ b/notes
@@ -1,1 +1,1 @@
-foo bar
+bar foo
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let del_idx = built
        .line_old_num
        .iter()
        .zip(built.line_new_num.iter())
        .position(|(o, n)| *o > 0 && *n == 0)
        .expect("del row present");
    let add_idx = built
        .line_new_num
        .iter()
        .zip(built.line_old_num.iter())
        .position(|(n, o)| *n > 0 && *o == 0)
        .expect("add row present");
    let del = body_content_char_bgs(&built.lines[del_idx]);
    let add = body_content_char_bgs(&built.lines[add_idx]);

    let uniform =
        |bgs: &[(char, Option<ratatui::style::Color>)], range: std::ops::Range<usize>| -> bool {
            let first = bgs[range.start].1;
            bgs[range].iter().all(|(_, b)| *b == first)
        };
    for row in [&del, &add] {
        assert!(uniform(row, 0..3), "first word must have uniform bg");
        assert!(uniform(row, 3..4), "space must have uniform bg");
        assert!(uniform(row, 4..7), "second word must have uniform bg");
    }

    for (c, bg) in &del {
        assert!(
            *bg == Some(diff_theme.del_bg) || *bg == Some(diff_theme.intra_del),
            "del char {c:?} unexpected bg {bg:?}"
        );
    }
    for (c, bg) in &add {
        assert!(
            *bg == Some(diff_theme.add_bg) || *bg == Some(diff_theme.intra_add),
            "add char {c:?} unexpected bg {bg:?}"
        );
    }
}

#[test]
fn intra_prefix_trim_avoids_middle_leak() {
    let (ss, theme) = test_assets();
    let diff_theme = crate::theme::app_theme().diff;
    let src = "\
diff --git a/notes b/notes
--- a/notes
+++ b/notes
@@ -1,1 +1,1 @@
-let x = old;
+let x = new;
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let del_idx = built
        .line_old_num
        .iter()
        .zip(built.line_new_num.iter())
        .position(|(o, n)| *o > 0 && *n == 0)
        .expect("del row present");
    let add_idx = built
        .line_new_num
        .iter()
        .zip(built.line_old_num.iter())
        .position(|(n, o)| *n > 0 && *o == 0)
        .expect("add row present");
    let del = body_content_char_bgs(&built.lines[del_idx]);
    let add = body_content_char_bgs(&built.lines[add_idx]);

    for (i, (c, bg)) in del.iter().enumerate() {
        let expected = if (8..11).contains(&i) {
            diff_theme.intra_del
        } else {
            diff_theme.del_bg
        };
        assert_eq!(
            *bg,
            Some(expected),
            "del char {i} {c:?}: got {bg:?} want {expected:?}"
        );
    }
    for (i, (c, bg)) in add.iter().enumerate() {
        let expected = if (8..11).contains(&i) {
            diff_theme.intra_add
        } else {
            diff_theme.add_bg
        };
        assert_eq!(
            *bg,
            Some(expected),
            "add char {i} {c:?}: got {bg:?} want {expected:?}"
        );
    }
}

#[test]
fn intra_suffix_trim_works() {
    let (ss, theme) = test_assets();
    let diff_theme = crate::theme::app_theme().diff;
    let src = "\
diff --git a/notes b/notes
--- a/notes
+++ b/notes
@@ -1,1 +1,1 @@
-foo();
+bar();
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let del_idx = built
        .line_old_num
        .iter()
        .zip(built.line_new_num.iter())
        .position(|(o, n)| *o > 0 && *n == 0)
        .expect("del row present");
    let add_idx = built
        .line_new_num
        .iter()
        .zip(built.line_old_num.iter())
        .position(|(n, o)| *n > 0 && *o == 0)
        .expect("add row present");
    let del = body_content_char_bgs(&built.lines[del_idx]);
    let add = body_content_char_bgs(&built.lines[add_idx]);

    for (i, (c, bg)) in del.iter().enumerate() {
        let expected = if i < 3 {
            diff_theme.intra_del
        } else {
            diff_theme.del_bg
        };
        assert_eq!(
            *bg,
            Some(expected),
            "del char {i} {c:?}: got {bg:?} want {expected:?}"
        );
    }
    for (i, (c, bg)) in add.iter().enumerate() {
        let expected = if i < 3 {
            diff_theme.intra_add
        } else {
            diff_theme.add_bg
        };
        assert_eq!(
            *bg,
            Some(expected),
            "add char {i} {c:?}: got {bg:?} want {expected:?}"
        );
    }
}

#[test]
fn build_split_lines_produces_side_by_side_row() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,1 +1,1 @@
-old
+new
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_split_lines(&files, &ss, &theme);

    let joined: Vec<String> = built
        .lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.to_string()).collect())
        .collect();
    let has_pair = joined
        .iter()
        .any(|s| s.contains("- old") && s.contains("+ new") && s.contains('│'));
    assert!(
        has_pair,
        "expected a framed `│- old│+ new` split row, got: {joined:?}"
    );
}

#[test]
fn build_split_lines_marks_del_only_and_add_only_rows() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,3 +1,3 @@
 keep
-drop
+ins
+extra
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_split_lines(&files, &ss, &theme);
    let joined: Vec<String> = built
        .lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.to_string()).collect())
        .collect();

    assert!(joined
        .iter()
        .any(|s| s.contains("- drop") && s.contains("+ ins")));

    assert!(joined.iter().any(|s| s.contains("+ extra")));
}

#[test]
fn build_split_lines_emits_top_frame_row() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,1 +1,1 @@
-a
+b
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_split_lines(&files, &ss, &theme);

    let first_line: String = built.lines[0]
        .spans
        .iter()
        .map(|s| s.content.to_string())
        .collect();
    assert!(
        first_line.starts_with("┌─"),
        "expected split view to start with a `┌─` frame row, got {first_line:?}"
    );

    assert_eq!(built.file_top_idx.first().copied(), Some(0));
}

#[test]
fn build_split_lines_emits_hunk_header_row() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,1 +1,1 @@
-a
+b
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_split_lines(&files, &ss, &theme);
    let joined: Vec<String> = built
        .lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.to_string()).collect())
        .collect();
    assert!(
        joined.iter().any(|s| s.starts_with("├─ around")),
        "expected a `├─ around` hunk header row, got: {joined:?}"
    );
}

#[test]
fn build_split_lines_bottom_row_has_tees_at_middle_and_gutters_when_numbers_visible() {
    use crate::app::{App, DiffSpec};
    use ratatui::{backend::TestBackend, Terminal};
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,1 +1,1 @@
-a
+b
";
    let files = crate::diff::parse_unified_diff(src);
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
    app.set_source_for_test(src.to_string());
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    app.build_split_now(&ss, &theme);

    app.diff_mut().unwrap().layout = crate::diff::DiffLayout::Split;
    app.set_line_numbers_visible(true);
    let mut terminal = Terminal::new(TestBackend::new(200, 12)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer().clone();

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
    let bottom_row = bottom_row.expect("expected a `└` bottom row in split view");
    let mut tees = 0usize;
    for x in 0..buffer.area.width {
        if buffer.cell((x, bottom_row)).unwrap().symbol() == "┴" {
            tees += 1;
        }
    }

    assert_eq!(
        tees, 3,
        "expected exactly 3 `┴` (middle + LEFT gutter + RIGHT gutter) on the split bottom row, got {tees}"
    );
}

#[test]
fn build_diff_lines_renders_combined_hunk_without_crash() {
    let (ss, theme) = test_assets();
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/diff/combined.diff");
    let src = std::fs::read_to_string(&fixture).expect("read combined fixture");
    let files = crate::diff::parse_unified_diff(&src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let combined_row_spans = built
        .lines
        .iter()
        .find(|l| {
            let text: String = l.spans.iter().map(|s| s.content.as_ref()).collect();
            text.contains("resolved") || text.contains("added-only-in-merge")
        })
        .expect("combined body line rendered")
        .spans
        .len();
    assert!(
        combined_row_spans >= 3,
        "expected ≥3 spans on combined line, got {combined_row_spans}"
    );
}

#[test]
fn build_diff_lines_renders_word_diff_segments() {
    let (ss, theme) = test_assets();
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/diff/word_diff.diff");
    let src = std::fs::read_to_string(&fixture).expect("read word_diff fixture");
    let files = crate::diff::parse_unified_diff(&src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let row = built
        .lines
        .iter()
        .find(|l| {
            let text: String = l.spans.iter().map(|s| s.content.as_ref()).collect();
            text.contains("the") && text.contains("test.")
        })
        .expect("word-diff row rendered");

    assert!(
        row.spans.len() >= 4,
        "expected multi-span word-diff row, got {} spans: {:?}",
        row.spans.len(),
        row.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
    );
}

#[test]
fn build_diff_lines_renders_nested_submodule_note() {
    let (ss, theme) = test_assets();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/diff/submodule_recursive.diff");
    let src = std::fs::read_to_string(&fixture).expect("read submodule_recursive fixture");
    let files = crate::diff::parse_unified_diff(&src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let joined: String = built
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("submodule contents"),
        "expected 'submodule contents' note in nested render: {joined:?}"
    );
    assert!(
        joined.contains("nested.rs"),
        "expected nested file path in render: {joined:?}"
    );
}
