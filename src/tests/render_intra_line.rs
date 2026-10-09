use super::common::render_helpers::*;
use super::test_assets;

#[test]
fn intra_line_highlights_added_chars() {
    let (ss, theme) = test_assets();
    let diff_theme = crate::theme::app_theme().diff;
    let src = "\
diff --git a/notes b/notes
--- a/notes
+++ b/notes
@@ -1,1 +1,1 @@
-foo bar
+foo bar baz
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let add_idx = built
        .line_new_num
        .iter()
        .zip(built.line_old_num.iter())
        .position(|(n, o)| *n > 0 && *o == 0)
        .expect("add row present");
    let bgs = body_content_char_bgs(&built.lines[add_idx]);
    let text: String = bgs.iter().map(|(c, _)| *c).collect();
    assert_eq!(text, "foo bar baz", "unexpected add content: {text:?}");
    for (i, (c, bg)) in bgs.iter().enumerate() {
        let expected = if i < 7 {
            diff_theme.add_bg
        } else {
            diff_theme.intra_add
        };
        assert_eq!(
            *bg,
            Some(expected),
            "char {i} {c:?} bg mismatch: got {bg:?} expected {expected:?}"
        );
    }
}

#[test]
fn intra_line_highlights_removed_chars() {
    let (ss, theme) = test_assets();
    let diff_theme = crate::theme::app_theme().diff;
    let src = "\
diff --git a/notes b/notes
--- a/notes
+++ b/notes
@@ -1,1 +1,1 @@
-foo bar baz
+foo bar
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let del_idx = built
        .line_old_num
        .iter()
        .zip(built.line_new_num.iter())
        .position(|(o, n)| *o > 0 && *n == 0)
        .expect("del row present");
    let bgs = body_content_char_bgs(&built.lines[del_idx]);
    let text: String = bgs.iter().map(|(c, _)| *c).collect();
    assert_eq!(text, "foo bar baz", "unexpected del content: {text:?}");
    for (i, (c, bg)) in bgs.iter().enumerate() {
        let expected = if i < 7 {
            diff_theme.del_bg
        } else {
            diff_theme.intra_del
        };
        assert_eq!(
            *bg,
            Some(expected),
            "char {i} {c:?} bg mismatch: got {bg:?} expected {expected:?}"
        );
    }
}

#[test]
fn intra_line_skips_when_no_common_chars() {
    let (ss, theme) = test_assets();
    let diff_theme = crate::theme::app_theme().diff;
    let src = "\
diff --git a/notes b/notes
--- a/notes
+++ b/notes
@@ -1,1 +1,1 @@
-aaa
+bbb
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
    for (c, bg) in body_content_char_bgs(&built.lines[del_idx]) {
        assert_eq!(
            bg,
            Some(diff_theme.del_bg),
            "del char {c:?} must keep uniform del_bg"
        );
    }
    for (c, bg) in body_content_char_bgs(&built.lines[add_idx]) {
        assert_eq!(
            bg,
            Some(diff_theme.add_bg),
            "add char {c:?} must keep uniform add_bg"
        );
    }
}

#[test]
fn intra_line_no_pair_leaves_uniform_bg() {
    let (ss, theme) = test_assets();
    let diff_theme = crate::theme::app_theme().diff;
    let src = "\
diff --git a/notes b/notes
--- a/notes
+++ b/notes
@@ -1,1 +1,2 @@
 ctx
+brand new line
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let add_idx = built
        .line_new_num
        .iter()
        .zip(built.line_old_num.iter())
        .position(|(n, o)| *n > 0 && *o == 0)
        .expect("add row present");
    for (c, bg) in body_content_char_bgs(&built.lines[add_idx]) {
        assert_eq!(
            bg,
            Some(diff_theme.add_bg),
            "solo add char {c:?} must keep uniform add_bg"
        );
    }
}
