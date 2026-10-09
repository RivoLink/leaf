use super::common::render_helpers::*;
use super::test_assets;
use crate::render::FrameDecorations;

#[test]
fn file_header_shows_status_and_counts() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,1 +1,3 @@
 ctx
+one
+two
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let idx = built
        .lines
        .iter()
        .position(|l| line_text(l).starts_with("┌─ "))
        .expect("file top frame present");
    let path_text = line_text(&built.lines[idx]);
    assert!(
        !path_text.contains('►'),
        "file top frame must not contain the `►` marker, got {path_text:?}"
    );
    let spans = built.file_header_status[idx]
        .as_ref()
        .expect("status cluster present for file-top row");
    let status: String = spans.iter().flat_map(|s| s.content.chars()).collect();
    assert!(
        status.contains("modified"),
        "expected \"modified\" status in top frame, got {status:?}"
    );
    assert!(
        status.contains("+2 −0"),
        "expected \"+2 −0\" counts in top frame, got {status:?}"
    );
}

#[test]
fn file_header_includes_zero_counts_for_binary() {
    use ratatui::style::Modifier;
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/blob b/blob
Binary files a/blob and b/blob differ
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let diff_theme = diff_theme_for_test();
    let idx = built
        .lines
        .iter()
        .position(|l| line_text(l).starts_with("┌─ "))
        .expect("file top frame present");
    let spans = built.file_header_status[idx]
        .as_ref()
        .expect("status cluster present for file-top row");
    let status: String = spans.iter().flat_map(|s| s.content.chars()).collect();
    assert!(
        status.contains("binary"),
        "expected \"binary\" status in top frame, got {status:?}"
    );
    assert!(
        status.contains("+0") && status.contains("−0"),
        "binary top frame now carries `+0 −0` for alignment, got {status:?}"
    );
    let label = &spans[0];
    assert_eq!(label.content, "binary");
    assert_eq!(
        label.style.fg,
        Some(diff_theme.hunk_header_fg),
        "binary label must use hunk_header_fg, got {:?}",
        label.style.fg
    );
    assert!(
        !label.style.add_modifier.contains(Modifier::BOLD),
        "binary label must not be BOLD, got {:?}",
        label.style.add_modifier
    );
}

#[test]
fn file_top_and_bottom_frame_present() {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/a b/a
--- a/a
+++ b/a
@@ -1,1 +1,2 @@
 keep
+first
diff --git a/b b/b
--- a/b
+++ b/b
@@ -1,1 +1,2 @@
 keep
+second
";
    let files = crate::diff::parse_unified_diff(src);
    assert_eq!(files.len(), 2);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let tops: usize = built
        .lines
        .iter()
        .filter(|l| line_text(l).starts_with("┌─ "))
        .count();
    let bottoms: usize = built
        .lines
        .iter()
        .filter(|l| line_text(l).trim_end() == "└─")
        .count();
    assert_eq!(tops, 2, "expected 2 file top frames, got {tops}");
    assert_eq!(bottoms, 2, "expected 2 file bottom frames, got {bottoms}");
}

#[test]
fn file_top_right_aligns_status_when_room() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled(
        "┌─ foo.rs ".to_string(),
        Style::default(),
    ))];
    let status: Vec<Option<Vec<Span<'static>>>> = vec![Some(vec![Span::styled(
        "modified · +1 −0".to_string(),
        Style::default(),
    )])];
    crate::render::close_frame_right(
        &mut lines,
        80,
        &theme,
        &FrameDecorations {
            status_by_line: Some(&status),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);
    assert_eq!(line_width_chars(&lines[0]), 80);
    assert_eq!(last_char(&lines[0]), Some('┐'));

    assert!(
        text.ends_with(" modified · +1 −0 ─┐"),
        "expected right-aligned status ending with `─┐`, got {text:?}"
    );
    let chars: Vec<char> = text.chars().collect();
    assert_eq!(
        chars[chars.len() - 2],
        '─',
        "second-to-last char must be `─`, got {text:?}"
    );
}

#[test]
fn file_top_no_status_falls_back_to_default_fill() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled(
        "┌─ foo.rs ".to_string(),
        Style::default(),
    ))];

    let status: Vec<Option<Vec<Span<'static>>>> = vec![None];
    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            status_by_line: Some(&status),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);
    assert_eq!(line_width_chars(&lines[0]), 40);
    assert_eq!(last_char(&lines[0]), Some('┐'));
    assert!(
        text.ends_with("─┐"),
        "no-status `┌` row should fill with `─` up to `┐`, got {text:?}"
    );
}

#[test]
fn file_top_falls_back_when_overflow() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled(
        "┌─ a/very/long/path.rs ".to_string(),
        Style::default(),
    ))];
    let status: Vec<Option<Vec<Span<'static>>>> = vec![Some(vec![Span::styled(
        "modified · +10 −10".to_string(),
        Style::default(),
    )])];

    crate::render::close_frame_right(
        &mut lines,
        30,
        &theme,
        &FrameDecorations {
            status_by_line: Some(&status),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);
    assert_eq!(last_char(&lines[0]), Some('┐'));

    assert!(
        text.contains("a/very/long/path.rs  modified · +10 −10 ┐"),
        "overflow fallback should be `path` + ` ` + `status` + ` ┐`, got {text:?}"
    );
}

#[test]
fn file_top_middle_fill_uses_horizontal_dash() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled(
        "┌─ foo.rs ".to_string(),
        Style::default(),
    ))];
    let status: Vec<Option<Vec<Span<'static>>>> = vec![Some(vec![Span::styled(
        "modified · +1 −0".to_string(),
        Style::default(),
    )])];
    crate::render::close_frame_right(
        &mut lines,
        80,
        &theme,
        &FrameDecorations {
            status_by_line: Some(&status),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);

    let mid: String = text
        .chars()
        .skip("┌─ foo.rs ".chars().count())
        .take_while(|c| *c == '─')
        .collect();
    assert!(
        !mid.is_empty(),
        "expected non-empty `─` fill between path and status, got {text:?}"
    );
    assert!(
        mid.chars().all(|c| c == '─'),
        "fill must be U+2500 `─`, got {mid:?}"
    );
}

#[test]
fn file_top_ends_with_two_char_corner() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled(
        "┌─ foo.rs ".to_string(),
        Style::default(),
    ))];
    let status: Vec<Option<Vec<Span<'static>>>> = vec![Some(vec![Span::styled(
        "modified · +1 −0".to_string(),
        Style::default(),
    )])];
    crate::render::close_frame_right(
        &mut lines,
        80,
        &theme,
        &FrameDecorations {
            status_by_line: Some(&status),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);
    assert_eq!(line_width_chars(&lines[0]), 80);
    let chars: Vec<char> = text.chars().collect();
    assert_eq!(chars[chars.len() - 2], '─');
    assert_eq!(chars[chars.len() - 1], '┐');
}

#[test]
fn bottom_border_has_gutter_join_when_line_numbers_visible() {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let mut lines = vec![Line::from(Span::styled("└─".to_string(), Style::default()))];

    crate::render::close_frame_right(
        &mut lines,
        40,
        &theme,
        &FrameDecorations {
            gutter_widths: Some((2usize, 2usize)),
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );
    let text = line_text(&lines[0]);
    assert_eq!(last_char(&lines[0]), Some('┘'));
    assert_eq!(line_width_chars(&lines[0]), 40);
    let chars: Vec<char> = text.chars().collect();
    assert_eq!(
        chars.get(6),
        Some(&'┴'),
        "expected `┴` at column 6 of the bottom row, got {text:?}"
    );
    assert_eq!(
        text.chars().filter(|c| *c == '┴').count(),
        1,
        "exactly one `┴` must appear on the bottom row, got {text:?}"
    );
}

#[test]
fn bottom_border_plain_fill_when_line_numbers_hidden() {
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
    assert_eq!(line_width_chars(&lines[0]), 40);
    assert!(
        !text.contains('┴'),
        "no `┴` must appear when line numbers are hidden, got {text:?}"
    );
}

#[test]
fn is_mode_change_with_hunks_detects_combined_case() {
    use crate::diff::{is_mode_change_with_hunks, parse_unified_diff};

    let pure = parse_unified_diff("diff --git a/x b/x\nold mode 100644\nnew mode 100755\n");
    assert!(!is_mode_change_with_hunks(&pure[0]));

    let combined = parse_unified_diff(
        "diff --git a/x b/x\nold mode 100644\nnew mode 100755\nindex 1111111..2222222 100755\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n",
    );
    assert!(is_mode_change_with_hunks(&combined[0]));

    let modified = parse_unified_diff(
        "diff --git a/x b/x\nindex 1111111..2222222 100644\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n",
    );
    assert!(!is_mode_change_with_hunks(&modified[0]));
}

#[test]
fn split_mode_plus_content_top_frame_has_middle_tee() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();
    let src = "diff --git a/scripts/x b/scripts/x\nold mode 100644\nnew mode 100755\nindex 1111111..2222222 100755\n--- a/scripts/x\n+++ b/scripts/x\n@@ -1 +1 @@\n-old\n+new\n";
    let files = parse_unified_diff(src);
    let built = crate::render::build_split_lines(&files, &ss, &theme);

    let top_idx = built.file_top_idx[0];
    assert!(
        !built.meta_only_frame_rows.contains(&top_idx),
        "combined top frame row must not be stripped (meta_only_frame_rows contains top_idx)"
    );

    let bottom_idx = built.file_bottom_idx[0];
    assert!(
        !built.meta_only_frame_rows.contains(&bottom_idx),
        "combined bottom frame row must not be stripped"
    );
}

#[test]
fn split_mode_plus_content_preserves_meta_prelude() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();
    let src = "diff --git a/scripts/x b/scripts/x\nold mode 100644\nnew mode 100755\nindex 1111111..2222222 100755\n--- a/scripts/x\n+++ b/scripts/x\n@@ -1 +1 @@\n-old\n+new\n";
    let files = parse_unified_diff(src);
    let built = crate::render::build_split_lines(&files, &ss, &theme);

    let fake_header = built
        .lines
        .iter()
        .find(|l| line_text(l).starts_with("├─ mode changed"))
        .expect("fake-header `├─ mode changed · +0 −0` must be emitted for combined");
    let text = line_text(fake_header);
    assert!(
        text.contains("+0") && text.contains("−0"),
        "fake-header counts must be forced to +0 −0 for combined, got {text:?}"
    );

    let note_idx = built
        .lines
        .iter()
        .position(|l| {
            let t = line_text(l);
            t.contains("executable permission added") || t.contains("mode changed")
        })
        .expect("bordered mode-change note must be emitted");
    assert!(
        built.single_column_body_rows.contains(&note_idx)
            || line_text(&built.lines[note_idx]).starts_with("├"),
        "bordered note row for combined must be tracked in single_column_body_rows"
    );
}

#[test]
fn split_pure_meta_uses_two_col_chrome() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();

    let cases: [(&str, &str); 2] = [
        (
            "diff --git a/img.png b/img.png\nindex 1111111..2222222 100644\nBinary files a/img.png and b/img.png differ\n",
            "binary content not shown",
        ),
        (
            "diff --git a/scripts/x b/scripts/x\nold mode 100644\nnew mode 100755\n",
            "executable permission added",
        ),
    ];

    for (src, note_text) in cases {
        let files = parse_unified_diff(src);
        let built = crate::render::build_split_lines(&files, &ss, &theme);

        let top_idx = built.file_top_idx[0];
        let bottom_idx = built.file_bottom_idx[0];
        assert!(
            !built.meta_only_frame_rows.contains(&top_idx),
            "{note_text}: top-frame must NOT be stripped (middle `┬` kept)"
        );

        assert!(
            built.meta_only_frame_rows.contains(&bottom_idx),
            "{note_text}: bottom-frame row must be tracked so phantom gutter `┴` tees are stripped"
        );
        let note_idx = built
            .lines
            .iter()
            .position(|l| line_text(l).contains(note_text))
            .unwrap_or_else(|| panic!("{note_text}: bordered note must be emitted"));
        assert!(
            built.single_column_body_rows.contains(&note_idx),
            "{note_text}: bordered note index must be tracked in single_column_body_rows"
        );
    }
}

#[test]
fn top_frame_label_mode_plus_content_is_modified() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();
    let src = "diff --git a/scripts/x b/scripts/x\nold mode 100644\nnew mode 100755\nindex 1111111..2222222 100755\n--- a/scripts/x\n+++ b/scripts/x\n@@ -1 +1 @@\n-old\n+new\n";
    let files = parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let top_idx = built
        .lines
        .iter()
        .position(|l| line_text(l).starts_with("┌─ "))
        .expect("file top frame present");
    let status = built.file_header_status[top_idx]
        .as_ref()
        .expect("status cluster present on top row");
    let text: String = status.iter().flat_map(|s| s.content.chars()).collect();
    assert!(
        text.contains("modified"),
        "combined top-frame status cluster must read `modified`, got {text:?}"
    );
    assert!(
        !text.contains("mode changed"),
        "combined top-frame must NOT read `mode changed` (that stays for pure mode-only), got {text:?}"
    );
}

#[test]
fn submodule_top_frame_label_is_subproject() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();
    let src = "diff --git a/vendor b/vendor\nindex bef0a93..3810002 160000\n--- a/vendor\n+++ b/vendor\n@@ -1 +1 @@\n-Subproject commit bef0a93b4abf0000000000000000000000000000\n+Subproject commit 381000237a5f0000000000000000000000000000\n";
    let files = parse_unified_diff(src);
    assert!(matches!(
        files[0].change,
        crate::diff::FileChange::Submodule
    ));
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let top_idx = built
        .lines
        .iter()
        .position(|l| line_text(l).starts_with("┌─ "))
        .expect("file top frame present");
    let status = built.file_header_status[top_idx]
        .as_ref()
        .expect("status cluster present on top row");
    let text: String = status.iter().flat_map(|s| s.content.chars()).collect();
    assert!(
        text.contains("subproject"),
        "submodule top-frame status cluster must read `subproject`, got {text:?}"
    );
    assert!(
        !text.contains("modified"),
        "submodule top-frame must NOT read `modified`, got {text:?}"
    );
}

#[test]
fn submodule_hunk_header_label_is_subproject() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();
    let src = "diff --git a/vendor b/vendor\nindex bef0a93..3810002 160000\n--- a/vendor\n+++ b/vendor\n@@ -1 +1 @@\n-Subproject commit bef0a93b4abf0000000000000000000000000000\n+Subproject commit 381000237a5f0000000000000000000000000000\n";
    let files = parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let header = built
        .lines
        .iter()
        .map(line_text)
        .find(|t| t.starts_with("├─ "))
        .expect("hunk header present");
    assert!(
        header.contains("subproject"),
        "submodule hunk header must contain `subproject`, got {header:?}"
    );
    assert!(
        !header.contains("around line"),
        "submodule hunk header must NOT contain `around line`, got {header:?}"
    );
}

#[test]
fn submodule_no_bordered_note_in_split() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();
    let src = "diff --git a/vendor b/vendor\nindex bef0a93..3810002 160000\n--- a/vendor\n+++ b/vendor\n@@ -1 +1 @@\n-Subproject commit bef0a93b4abf0000000000000000000000000000\n+Subproject commit 381000237a5f0000000000000000000000000000\n";
    let files = parse_unified_diff(src);
    let built = crate::render::build_split_lines(&files, &ss, &theme);
    let joined: String = built
        .lines
        .iter()
        .map(line_text)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !joined.contains("submodule change"),
        "split render must NOT contain the removed `submodule change` note, got {joined:?}"
    );
    assert!(
        joined.contains("Subproject commit"),
        "split render must preserve the `Subproject commit` hunk lines, got {joined:?}"
    );
}

#[test]
fn theme_presets_define_meta_note_colors() {
    use crate::theme::{theme_by_preset, ThemePreset};
    for preset in [
        ThemePreset::Arctic,
        ThemePreset::OceanDark,
        ThemePreset::Forest,
        ThemePreset::SolarizedDark,
    ] {
        let t = theme_by_preset(preset);

        assert_ne!(
            t.diff.meta_note_fg,
            ratatui::style::Color::Reset,
            "preset {preset:?} meta_note_fg must be defined"
        );
        assert_ne!(
            t.diff.meta_note_bg,
            ratatui::style::Color::Reset,
            "preset {preset:?} meta_note_bg must be defined"
        );
    }
}

#[test]
fn bordered_note_has_exclamation_prefix_and_blue_bg() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();

    let src = "diff --git a/img.png b/img.png\nindex 1111111..2222222 100644\nBinary files a/img.png and b/img.png differ\n";
    let files = parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let note = built
        .lines
        .iter()
        .find(|l| line_text(l).contains("binary content not shown"))
        .expect("binary bordered note must be emitted");

    assert!(
        note.spans.len() >= 3,
        "bordered note must have border + sigil + text spans, got {} spans",
        note.spans.len()
    );
    assert_eq!(
        note.spans[0].content.as_ref(),
        "│",
        "bordered note border span must be a bare `│` (no trailing space), got {:?}",
        note.spans[0].content
    );
    assert_eq!(
        note.spans[1].content.as_ref(),
        "!",
        "bordered note sigil must be `!` (second span), got {:?}",
        note.spans[1].content
    );
    let diff_theme = crate::theme::app_theme().diff;
    let meta_fg = diff_theme.meta_note_fg;
    let meta_bg = diff_theme.meta_note_bg;
    let sigil_fg = diff_theme.meta_note_sigil_fg;
    assert_eq!(
        note.spans[1].style.fg,
        Some(sigil_fg),
        "`!` sigil must use meta_note_sigil_fg"
    );
    assert_eq!(
        note.spans[1].style.bg,
        Some(meta_bg),
        "`!` sigil must use meta_note_bg"
    );
    assert_eq!(
        note.spans[2].style.fg,
        Some(meta_fg),
        "note text must use meta_note_fg"
    );
    assert_eq!(
        note.spans[2].style.bg,
        Some(meta_bg),
        "note text must use meta_note_bg"
    );
}

#[test]
fn preview_mode_plus_content_shows_full_delta() {
    use crate::app::{build_preview_pair, DiffSpec};
    use crate::diff::parse_unified_diff;

    let src = "diff --git a/scripts/x b/scripts/x\nold mode 100644\nnew mode 100755\nindex 1111111..2222222 100755\n--- a/scripts/x\n+++ b/scripts/x\n@@ -1,3 +1,3 @@\n ctx\n-old\n+new\n";
    let files = parse_unified_diff(src);
    assert_eq!(files.len(), 1);
    assert!(!files[0].hunks.is_empty());

    let (ss, theme) = test_assets();
    let pair = build_preview_pair(&files[0], &DiffSpec::default(), &ss, &theme);

    assert!(
        !pair.before_is_note,
        "combined preview: before side must NOT be flagged as a note"
    );
    assert!(
        !pair.after_is_note,
        "combined preview: after side must NOT be flagged as a note (delta visible)"
    );

    assert!(
        pair.after_lines.len() > 1,
        "combined preview after column must show content, not the collapsed mode-change note"
    );
}

const PURE_RENAME_DIFF: &str =
    "diff --git a/old.rs b/new.rs\nsimilarity index 100%\nrename from old.rs\nrename to new.rs\n";

const RENAME_PLUS_CONTENT_DIFF: &str = "diff --git a/old.rs b/new.rs\nsimilarity index 92%\nrename from old.rs\nrename to new.rs\nindex 1111111..2222222 100644\n--- a/old.rs\n+++ b/new.rs\n@@ -1 +1 @@\n-old\n+new\n";

#[test]
fn pure_rename_top_frame_shows_renamed_label() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();
    let src = PURE_RENAME_DIFF;
    let files = parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let top_idx = built.file_top_idx[0];
    let status_spans = built.file_header_status[top_idx]
        .as_ref()
        .expect("pure rename top-frame must carry a status cluster");
    let text: String = status_spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(
        text.contains("renamed") && !text.contains("renamed from"),
        "top-frame status label must be short `renamed`, got {text:?}"
    );
    assert!(
        text.contains("+0") && text.contains("−0"),
        "pure rename cluster must carry +0 −0, got {text:?}"
    );
}

#[test]
fn pure_rename_emits_meta_hunk_header_and_bordered_note_unified() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();
    let src = PURE_RENAME_DIFF;
    let files = parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let header = built
        .lines
        .iter()
        .find(|l| line_text(l).starts_with("├─ renamed"))
        .expect("fake-header `├─ renamed · +0 −0` must be emitted for pure rename");
    assert!(line_text(header).contains("+0") && line_text(header).contains("−0"));

    built
        .lines
        .iter()
        .find(|l| line_text(l).contains("renamed from old.rs"))
        .expect("bordered note `│!renamed from old.rs` must be emitted");
}

#[test]
fn pure_rename_emits_bordered_note_split() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();
    let src = PURE_RENAME_DIFF;
    let files = parse_unified_diff(src);
    let built = crate::render::build_split_lines(&files, &ss, &theme);

    let note_idx = built
        .lines
        .iter()
        .position(|l| line_text(l).contains("renamed from old.rs"))
        .expect("bordered rename note must be emitted in split");
    assert!(
        built.single_column_body_rows.contains(&note_idx),
        "bordered rename note row must be tracked in single_column_body_rows"
    );
}

#[test]
fn rename_plus_content_shows_modified_label_and_rename_prelude() {
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();
    let src = RENAME_PLUS_CONTENT_DIFF;
    let files = parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);

    let top_idx = built.file_top_idx[0];
    let status_spans = built.file_header_status[top_idx]
        .as_ref()
        .expect("rename+modified top-frame must carry a status cluster");
    let text: String = status_spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(
        text.contains("modified"),
        "rename+modified top-frame must read `modified`, got {text:?}"
    );
    assert!(
        !text.contains("renamed"),
        "rename+modified top-frame must NOT read `renamed` (that stays for pure rename), got {text:?}"
    );
    built
        .lines
        .iter()
        .find(|l| line_text(l).contains("renamed from old.rs"))
        .expect("rename+modified must still emit the rename bordered note as prelude");
}

#[test]
fn pure_rename_preview_mirrors_content_with_note_on_right() {
    use crate::app::{build_preview_pair, DiffSpec};
    use crate::diff::parse_unified_diff;
    let (ss, theme) = test_assets();
    let src = PURE_RENAME_DIFF;
    let files = parse_unified_diff(src);
    let pair = build_preview_pair(&files[0], &DiffSpec::default(), &ss, &theme);

    assert!(
        pair.after_is_note,
        "pure rename preview must mark AFTER as note"
    );
    let after_text: String = pair
        .after_lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(
        after_text.contains("renamed from old.rs"),
        "AFTER note must contain `renamed from old.rs`, got {after_text:?}"
    );
}
