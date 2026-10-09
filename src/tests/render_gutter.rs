use super::common::render_helpers::*;
use super::test_assets;

#[test]
fn gutter_uses_add_fg_for_add_line_new_number() {
    let (diff_theme, lines, state) = build_gutter_lines_for_test();
    let spans = find_gutter_line_for(&lines, &state, |o, n| n > 0 && o == 0);

    let old_span = &spans[1];
    let new_span = &spans[3];
    assert_eq!(
        old_span.style.fg,
        Some(diff_theme.frame_fg),
        "old column on an add line must stay frame_fg (blank), got {:?}",
        old_span.style.fg
    );
    assert!(
        old_span.content.trim().is_empty(),
        "old column on an add line must be blank, got {:?}",
        old_span.content
    );
    assert_eq!(
        new_span.style.fg,
        Some(diff_theme.add_fg),
        "new-number span on an add line must use add_fg, got {:?}",
        new_span.style.fg
    );
    assert!(
        !new_span.content.trim().is_empty(),
        "new-number span on an add line must carry the number, got {:?}",
        new_span.content
    );
}

#[test]
fn gutter_uses_del_fg_for_del_line_old_number() {
    let (diff_theme, lines, state) = build_gutter_lines_for_test();
    let spans = find_gutter_line_for(&lines, &state, |o, n| o > 0 && n == 0);
    let old_span = &spans[1];
    let new_span = &spans[3];
    assert_eq!(
        old_span.style.fg,
        Some(diff_theme.del_fg),
        "old-number span on a del line must use del_fg, got {:?}",
        old_span.style.fg
    );
    assert!(
        !old_span.content.trim().is_empty(),
        "old-number span on a del line must carry the number, got {:?}",
        old_span.content
    );
    assert_eq!(
        new_span.style.fg,
        Some(diff_theme.frame_fg),
        "new column on a del line must stay frame_fg (blank), got {:?}",
        new_span.style.fg
    );
    assert!(
        new_span.content.trim().is_empty(),
        "new column on a del line must be blank, got {:?}",
        new_span.content
    );
}

#[test]
fn gutter_stays_frame_fg_for_context_line() {
    let (diff_theme, lines, state) = build_gutter_lines_for_test();
    let spans = find_gutter_line_for(&lines, &state, |o, n| o > 0 && n > 0);
    let old_span = &spans[1];
    let sep_span = &spans[2];
    let new_span = &spans[3];
    let pipe_sep_span = &spans[4];
    let sigil_span = &spans[5];
    let gap_span = &spans[6];
    assert_eq!(old_span.style.fg, Some(diff_theme.frame_fg));
    assert_eq!(sep_span.style.fg, Some(diff_theme.frame_fg));
    assert_eq!(new_span.style.fg, Some(diff_theme.frame_fg));
    assert_eq!(pipe_sep_span.style.fg, Some(diff_theme.frame_fg));
    assert_eq!(sep_span.content, " ");
    assert_eq!(pipe_sep_span.content, "│");
    assert_eq!(gap_span.content, " ");

    assert_eq!(sigil_span.content, " ");
    assert_eq!(sigil_span.style.fg, None);
    assert_eq!(sigil_span.style.bg, None);

    assert_eq!(gap_span.style.fg, None);
}

#[test]
fn gutter_widths_match_max_digit_count() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -98,3 +98,4 @@
 keep
-old
+new
 tail
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let mut state = crate::app::DiffState::new(std::sync::Arc::new(files), Default::default());
    state.hunk_ranges = built.hunk_ranges.clone();
    state.line_owner = built.line_owner.clone();
    state.line_old_num = built.line_old_num.clone();
    state.line_new_num = built.line_new_num.clone();
    state.file_header_status = built.file_header_status.clone();
    let mut lines = built.lines.clone();
    let diff_theme = diff_theme_for_test();
    crate::render::apply_number_gutter(&mut lines, &state, 0, &diff_theme);

    let digit_width_old = 101usize.to_string().len();
    let digit_width_new = 102usize.to_string().len();
    for (i, line) in lines.iter().enumerate() {
        let first = line.spans.first();

        let is_gutter_body = first.is_some_and(|s| s.content == "│") && line.spans.len() >= 7;
        if !is_gutter_body {
            continue;
        }
        let o = state.line_old_num.get(i).copied().unwrap_or(0);
        let n = state.line_new_num.get(i).copied().unwrap_or(0);
        let old_span = &line.spans[1];
        let new_span = &line.spans[3];
        let pipe_sep_span = &line.spans[4];
        assert_eq!(
            old_span.content.chars().count(),
            digit_width_old,
            "old column must be exactly {digit_width_old} chars, got {:?}",
            old_span.content
        );
        assert_eq!(
            new_span.content.chars().count(),
            digit_width_new,
            "new column must be exactly {digit_width_new} chars, got {:?}",
            new_span.content
        );

        assert_eq!(first.unwrap().content, "│");
        assert_eq!(
            pipe_sep_span.content, "│",
            "`│` separator must sit immediately after the new-number span"
        );
        assert_eq!(
            line.spans[5].content.chars().count(),
            1,
            "sigil span (post-gutter) must be a single char, got {:?}",
            line.spans[5].content
        );
        if o == 0 {
            assert_eq!(
                old_span.content,
                " ".repeat(digit_width_old),
                "blank old column must be exactly {digit_width_old} spaces"
            );
        }
        if n == 0 {
            assert_eq!(
                new_span.content,
                " ".repeat(digit_width_new),
                "blank new column must be exactly {digit_width_new} spaces"
            );
        }
    }
}

#[test]
fn gutter_columns_can_have_different_widths() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -997,2 +997,4 @@
 keep
-old
+new1
+new2
+new3
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let mut state = crate::app::DiffState::new(std::sync::Arc::new(files), Default::default());
    state.hunk_ranges = built.hunk_ranges.clone();
    state.line_owner = built.line_owner.clone();
    state.line_old_num = built.line_old_num.clone();
    state.line_new_num = built.line_new_num.clone();
    state.file_header_status = built.file_header_status.clone();
    let mut lines = built.lines.clone();
    let diff_theme = diff_theme_for_test();
    crate::render::apply_number_gutter(&mut lines, &state, 0, &diff_theme);

    let expected_old_width = 3usize;
    let expected_new_width = 4usize;
    assert_ne!(
        expected_old_width, expected_new_width,
        "test invariant: this diff must exercise DIFFERENT per-column widths"
    );

    let mut seen = false;
    for line in lines.iter() {
        let first = line.spans.first();
        let is_gutter_body = first.is_some_and(|s| s.content == "│") && line.spans.len() >= 7;
        if !is_gutter_body {
            continue;
        }
        let old_span = &line.spans[1];
        let new_span = &line.spans[3];
        assert_eq!(
            old_span.content.chars().count(),
            expected_old_width,
            "old column must be exactly {expected_old_width} chars, got {:?}",
            old_span.content
        );
        assert_eq!(
            new_span.content.chars().count(),
            expected_new_width,
            "new column must be exactly {expected_new_width} chars, got {:?}",
            new_span.content
        );
        seen = true;
    }
    assert!(seen, "expected at least one bordered body row to check");
}

#[test]
fn gutter_total_width_is_exactly_two_columns_plus_one_space() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -98,3 +98,4 @@
 keep
-old
+new
 tail
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let mut state = crate::app::DiffState::new(std::sync::Arc::new(files), Default::default());
    state.hunk_ranges = built.hunk_ranges.clone();
    state.line_owner = built.line_owner.clone();
    state.line_old_num = built.line_old_num.clone();
    state.line_new_num = built.line_new_num.clone();
    state.file_header_status = built.file_header_status.clone();
    let mut lines = built.lines.clone();
    let diff_theme = diff_theme_for_test();
    crate::render::apply_number_gutter(&mut lines, &state, 0, &diff_theme);

    let digit_width_old = 101usize.to_string().len();
    let digit_width_new = 102usize.to_string().len();
    let expected = digit_width_old + 1 + digit_width_new;
    let mut seen = false;
    for line in lines.iter() {
        let first = line.spans.first();
        let is_gutter_body = first.is_some_and(|s| s.content == "│") && line.spans.len() >= 7;
        if !is_gutter_body {
            continue;
        }
        let total: usize = line.spans[1..=3]
            .iter()
            .map(|s| s.content.chars().count())
            .sum();
        assert_eq!(
            total, expected,
            "gutter content width must be exactly {expected} chars (digit_width_old + 1 + digit_width_new)"
        );

        assert_eq!(
            line.spans[4].content, "│",
            "expected `│` separator right after the new-number span"
        );
        assert_eq!(
            line.spans[5].content.chars().count(),
            1,
            "sigil must sit immediately after the right `│` separator"
        );
        seen = true;
    }
    assert!(seen, "expected at least one bordered body row to check");
}

#[test]
fn gutter_has_separator_pipe_before_sigil() {
    let (diff_theme, lines, state) = build_gutter_lines_for_test();

    let spans = find_gutter_line_for(&lines, &state, |_o, _n| true);
    let new_span = &spans[3];
    let pipe_sep_span = &spans[4];
    let sigil_span = &spans[5];
    assert!(
        !new_span.content.is_empty(),
        "new-number span must carry the digit-width content, got {:?}",
        new_span.content
    );
    assert_eq!(
        pipe_sep_span.content, "│",
        "expected `│` separator span immediately after the new-number span"
    );
    assert_eq!(
        pipe_sep_span.style.fg,
        Some(diff_theme.frame_fg),
        "`│` separator must carry frame_fg, got {:?}",
        pipe_sep_span.style.fg
    );

    assert_eq!(pipe_sep_span.style.bg, None);
    assert_eq!(
        sigil_span.content.chars().count(),
        1,
        "sigil span must be a single char, got {:?}",
        sigil_span.content
    );
}
