use crate::app::{App, AppConfig, AppMode, DiffSource, DiffSpec, PreviewPair};
use crate::diff::{parse_unified_diff, DiffLayout};
use ratatui::text::Line;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

fn make_app() -> App {
    App::new(
        Vec::<Line<'static>>::new(),
        Vec::new(),
        "stdin".to_string(),
        false,
        false,
        None,
        None,
    )
}

fn syntect_defaults() -> (SyntaxSet, syntect::highlighting::Theme) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = ts.themes["base16-ocean.dark"].clone();
    (ss, theme)
}

#[test]
fn install_diff_switches_mode_and_populates_state() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-old\n+new\n";
    let files = parse_unified_diff(src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    assert_eq!(app.mode(), AppMode::Diff);
    let state = app.diff().expect("diff state installed");
    assert_eq!(state.files.len(), 1);
    assert_eq!(state.hunk_ranges.len(), 1);
    assert!(!state.line_owner.is_empty());
}

#[test]
fn diff_next_file_moves_across_files() {
    let src = "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-x\n+X\ndiff --git a/b b/b\n--- a/b\n+++ b/b\n@@ -1 +1 @@\n-y\n+Y\n";
    let files = parse_unified_diff(src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    assert!(app.diff_next_file());
    assert_eq!(app.diff().unwrap().current_file_idx, 1);
    assert!(!app.diff_next_file());
}

#[test]
fn diff_spec_source_defaults_to_working() {
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        Vec::new(),
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    assert_eq!(app.diff().unwrap().spec.source, DiffSource::Working);
}

#[test]
fn install_diff_creates_no_hunks_when_diff_is_empty() {
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        Vec::new(),
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    let state = app.diff().unwrap();
    assert!(state.hunk_ranges.is_empty());
    assert!(state.files.is_empty());
}

#[test]
fn install_diff_enables_line_numbers_by_default() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
    let files = parse_unified_diff(src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();

    assert!(!app.is_line_number_visible());
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    assert!(app.is_line_number_visible());
}

#[allow(dead_code)]
fn _config_placeholder(_c: AppConfig) {}

#[test]
fn diff_toggle_tree_flips_visibility() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
    let files = parse_unified_diff(src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    assert!(!app.diff().unwrap().tree_visible);
    app.diff_toggle_tree();
    assert!(app.diff().unwrap().tree_visible);
    app.diff_toggle_tree();
    assert!(!app.diff().unwrap().tree_visible);
}

#[test]
fn diff_toggle_layout_flips_unified_split() {
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        Vec::new(),
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    assert_eq!(app.diff().unwrap().layout, DiffLayout::Unified);
    app.build_split_now(&ss, &theme);
    app.diff_toggle_layout(&ss, &theme);
    assert_eq!(app.diff().unwrap().layout, DiffLayout::Split);
    app.diff_toggle_layout(&ss, &theme);
    assert_eq!(app.diff().unwrap().layout, DiffLayout::Unified);
}

#[test]
fn install_diff_preserves_view_state_across_reinstall() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
    let files = parse_unified_diff(src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files.clone(),
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    app.diff_toggle_tree();
    app.build_split_now(&ss, &theme);
    app.diff_toggle_layout(&ss, &theme);

    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    assert!(app.diff().unwrap().tree_visible);
    assert_eq!(app.diff().unwrap().layout, DiffLayout::Split);
}

#[test]
fn install_diff_builds_split_lines_alongside_unified() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n-old1\n-old2\n+new1\n+new2\n";
    let files = parse_unified_diff(src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    app.build_split_now(&ss, &theme);
    let split = &app.diff().unwrap().split_lines;
    assert!(!split.is_empty(), "split-view lines should be built");

    let has_divider = split.iter().any(|line| {
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        text.chars().filter(|c| *c == '│').count() >= 2
    });
    assert!(has_divider, "split lines must include the `│` divider");
}

#[test]
fn sync_render_width_preserves_diff_lines() {
    let src = "diff --git a/x.rs b/x.rs\nindex 111..222 100644\n--- a/x.rs\n+++ b/x.rs\n@@ -1,3 +1,4 @@\n ctx\n-old\n+new1\n+new2\n ctx2\n";
    let files = parse_unified_diff(src);
    let mut app = App::new_with_source(
        Vec::<Line<'static>>::new(),
        Vec::new(),
        AppConfig {
            filename: "stdin".to_string(),
            source: src.to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );

    assert_eq!(app.mode(), AppMode::Diff);
    let first = app
        .line(0)
        .expect("at least one rendered line after install_diff");
    let first_text: String = first.spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(
        first_text.starts_with("┌─ "),
        "expected human-readable file top frame before sync, got {first_text:?}"
    );

    let ts = ThemeSet::load_defaults();
    let changed = app.sync_render_width(120, &ss, &ts);
    assert!(changed, "render_width should have changed from default 80");

    assert_eq!(app.mode(), AppMode::Diff, "mode must remain Diff");
    let first_after = app
        .line(0)
        .expect("diff line buffer must still be populated");
    let first_after_text: String = first_after
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert!(
        first_after_text.starts_with("┌─ "),
        "expected diff file top frame to survive sync_render_width, got {first_after_text:?}"
    );

    for idx in 0..app.total() {
        if let Some(line) = app.line(idx) {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(
                !text.contains("diff --git"),
                "line {idx} contains raw 'diff --git' header, buffer was overwritten: {text:?}"
            );
        }
    }
}

#[test]
fn diff_toggle_preview_flips_flag_on_markdown_file() {
    let src = "diff --git a/README.md b/README.md\n--- a/README.md\n+++ b/README.md\n@@ -1 +1 @@\n-# Old\n+# New\n";
    let files = parse_unified_diff(src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.set_source_for_test(src.to_string());
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    assert!(!app.diff().unwrap().preview_visible);
    let flipped = app.diff_toggle_preview(&ss, &theme);
    assert!(flipped);
    assert!(app.diff().unwrap().preview_visible);

    assert!(app.diff().unwrap().preview_cache.contains_key(&0));

    app.diff_toggle_preview(&ss, &theme);
    assert!(!app.diff().unwrap().preview_visible);
}

#[test]
fn diff_toggle_preview_works_on_non_markdown_file() {
    let src =
        "diff --git a/main.rs b/main.rs\n--- a/main.rs\n+++ b/main.rs\n@@ -1 +1 @@\n-old\n+new\n";
    let files = parse_unified_diff(src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.set_source_for_test(src.to_string());
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    let flipped = app.diff_toggle_preview(&ss, &theme);
    assert!(flipped);
    assert!(app.diff().unwrap().preview_visible);
    assert!(app.diff().unwrap().preview_cache.contains_key(&0));
}

#[test]
fn diff_toggle_preview_cache_hit_avoids_rebuild() {
    let src = "diff --git a/README.md b/README.md\n--- a/README.md\n+++ b/README.md\n@@ -1 +1 @@\n-# Old\n+# New\n";
    let files = parse_unified_diff(src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.set_source_for_test(src.to_string());
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    app.diff_toggle_preview(&ss, &theme);

    let sentinel = PreviewPair {
        before_lines: Vec::new(),
        after_lines: Vec::new(),
        before_is_note: false,
        after_is_note: false,
        after_error: Some("SENTINEL".to_string()),
    };
    app.diff_mut()
        .unwrap()
        .preview_cache
        .insert(0, sentinel.clone());

    app.diff_toggle_preview(&ss, &theme);
    app.diff_toggle_preview(&ss, &theme);
    let cached = app.diff().unwrap().preview_cache.get(&0).cloned().unwrap();
    assert_eq!(cached.after_error.as_deref(), Some("SENTINEL"));
    assert!(cached.before_lines.is_empty());
    assert!(cached.after_lines.is_empty());
}

#[test]
fn install_diff_invalidates_preview_cache() {
    let src = "diff --git a/README.md b/README.md\n--- a/README.md\n+++ b/README.md\n@@ -1 +1 @@\n-# Old\n+# New\n";
    let files = parse_unified_diff(src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.set_source_for_test(src.to_string());
    app.install_diff(
        files.clone(),
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    app.diff_toggle_preview(&ss, &theme);
    assert!(app.diff().unwrap().preview_cache.contains_key(&0));
    assert!(app.diff().unwrap().preview_visible);

    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    assert!(app.diff().unwrap().preview_cache.is_empty());
    assert!(!app.diff().unwrap().preview_visible);
}

fn multi_file_diff_src(n: usize) -> String {
    let mut out = String::new();
    for i in 0..n {
        out.push_str(&format!(
            "diff --git a/f{i}.rs b/f{i}.rs\n--- a/f{i}.rs\n+++ b/f{i}.rs\n@@ -1 +1 @@\n-old{i}\n+new{i}\n",
        ));
    }
    out
}

#[test]
fn diff_current_file_from_scroll_follows_passive_scroll() {
    let src = multi_file_diff_src(12);
    let files = parse_unified_diff(&src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    assert_eq!(app.diff().unwrap().current_file_idx, 0);
    let top_of_file_2 = app.diff().unwrap().file_top_idx[2];
    app.scroll_to(top_of_file_2);
    assert_eq!(app.diff_current_file_from_scroll(), Some(2));

    assert_eq!(app.diff().unwrap().current_file_idx, 2);
}

#[test]
fn diff_current_file_from_scroll_updates_when_n_pressed() {
    let src = multi_file_diff_src(12);
    let files = parse_unified_diff(&src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );

    let top_of_file_2 = app.diff().unwrap().file_top_idx[2];
    app.scroll_to(top_of_file_2);
    app.diff_mut().unwrap().current_file_idx = 2;
    assert!(app.diff_next_file());
    assert_eq!(app.diff().unwrap().current_file_idx, 3);
    assert_eq!(app.diff_current_file_from_scroll(), Some(3));
}

#[test]
fn diff_current_file_from_scroll_falls_forward_between_files() {
    let src = multi_file_diff_src(4);
    let files = parse_unified_diff(&src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    let bottom_of_file_1 = app.diff().unwrap().file_bottom_idx[1];

    app.scroll_to(bottom_of_file_1 + 1);

    let fi = app.diff_current_file_from_scroll().expect("some file");
    assert!(fi == 1 || fi == 2, "expected file 1 or 2, got {fi}");
}

#[test]
fn diff_current_file_from_scroll_at_start_returns_first_file() {
    let src = multi_file_diff_src(3);
    let files = parse_unified_diff(&src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );

    app.scroll_to(0);
    assert_eq!(app.diff_current_file_from_scroll(), Some(0));
}

#[test]
fn diff_current_file_from_scroll_middle_of_file_2_returns_1() {
    let src = multi_file_diff_src(4);
    let files = parse_unified_diff(&src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    let top_f1 = app.diff().unwrap().file_top_idx[1];
    let bot_f1 = app.diff().unwrap().file_bottom_idx[1];
    let mid = (top_f1 + bot_f1) / 2;
    app.scroll_to(mid);
    assert_eq!(app.diff_current_file_from_scroll(), Some(1));
}

#[test]
fn diff_status_cache_refreshes_on_diff_scroll() {
    let src = multi_file_diff_src(6);
    let files = parse_unified_diff(&src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );

    app.refresh_status_cache(50);
    let first = app.status_line().clone();

    let top_f3 = app.diff().unwrap().file_top_idx[3];
    app.scroll_to(top_f3);

    app.refresh_status_cache(50);
    let second = app.status_line().clone();

    let first_text: String = first.spans.iter().map(|s| s.content.clone()).collect();
    let second_text: String = second.spans.iter().map(|s| s.content.clone()).collect();
    assert!(
        first_text.contains("1/6 files"),
        "expected first status to show 1/6 files, got: {first_text}",
    );
    assert!(
        second_text.contains("4/6 files"),
        "expected second status to show 4/6 files, got: {second_text}",
    );
}

#[test]
fn reconstruct_after_from_hunks_applies_add_and_del() {
    let src = concat!(
        "diff --git a/x.md b/x.md\n",
        "--- a/x.md\n",
        "+++ b/x.md\n",
        "@@ -1,3 +1,3 @@\n",
        " top\n",
        "-old\n",
        "+new\n",
        " bottom\n",
    );
    let files = parse_unified_diff(src);
    let before = "top\nold\nbottom\n";
    let after = crate::app::reconstruct_after_from_hunks(before, &files[0]).unwrap();
    assert!(after.contains("new"));
    assert!(!after.contains("old"));
    assert!(after.contains("top"));
    assert!(after.contains("bottom"));
}

fn preview_error_app() -> (App, SyntaxSet, syntect::highlighting::Theme) {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-old\n+new\n";
    let files = parse_unified_diff(src);
    let mut app = make_app();
    let (ss, theme) = syntect_defaults();
    app.set_source_for_test(src.to_string());
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    (app, ss, theme)
}

fn preview_pair_with_error(err: &str) -> PreviewPair {
    PreviewPair {
        before_lines: Vec::new(),
        after_lines: Vec::new(),
        before_is_note: false,
        after_is_note: false,
        after_error: Some(err.to_string()),
    }
}

#[test]
fn is_diff_preview_reconstruction_error_true_when_cache_has_error() {
    let (mut app, ss, theme) = preview_error_app();
    app.diff_toggle_preview(&ss, &theme);
    app.diff_mut()
        .unwrap()
        .preview_cache
        .insert(0, preview_pair_with_error("boom"));
    assert!(app.is_diff_preview_reconstruction_error());
}

#[test]
fn is_diff_preview_reconstruction_error_false_when_preview_closed() {
    let (mut app, _ss, _theme) = preview_error_app();
    app.diff_mut()
        .unwrap()
        .preview_cache
        .insert(0, preview_pair_with_error("boom"));
    assert!(!app.is_diff_preview_reconstruction_error());
}

#[test]
fn is_diff_preview_reconstruction_error_false_when_cache_empty() {
    let (mut app, ss, theme) = preview_error_app();
    app.diff_toggle_preview(&ss, &theme);
    app.diff_mut().unwrap().preview_cache.clear();
    assert!(!app.is_diff_preview_reconstruction_error());
}

#[test]
fn status_bar_shows_partial_preview_badge_when_reconstruction_fails() {
    use crate::render::build_status_bar;
    let (mut app, ss, theme) = preview_error_app();
    app.diff_toggle_preview(&ss, &theme);
    app.diff_mut().unwrap().preview_cache.insert(
        0,
        preview_pair_with_error("hunk start 10 past before EOF (6 lines)"),
    );
    let spans = build_status_bar(&app, 0);
    let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(
        text.contains("Partial preview, source file unavailable"),
        "status bar should show badge, got: {text}",
    );
}
