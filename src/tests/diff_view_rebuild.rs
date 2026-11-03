use super::{test_assets, unique_temp_dir, write_theme_test_state};
use crate::app::{App, AppConfig, DiffSpec};
use crate::diff::{parse_unified_diff, DiffLayout};
use crate::theme::{current_theme_selection, set_theme_preset, set_theme_selection, ThemePreset};
use ratatui::text::Line;

const SRC: &str = "\
diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,2 +1,2 @@
 # title
-let a = 1;
+let a = 2;
diff --git a/b.rs b/b.rs
--- a/b.rs
+++ b/b.rs
@@ -1 +1 @@
-old
+new
";

fn plain_lines(app: &App) -> Vec<String> {
    app.visible_lines(0, app.total())
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
        .collect()
}

fn diff_app(src: &str, filepath: Option<std::path::PathBuf>) -> App {
    let (ss, theme) = test_assets();
    let mut app = App::new_with_source(
        Vec::<Line<'static>>::new(),
        Vec::new(),
        AppConfig {
            filename: "test.diff".to_string(),
            source: src.to_string(),
            debug_input: false,
            watch: false,
            filepath,
            last_file_state: None,
        },
    );
    app.install_diff(
        parse_unified_diff(src),
        DiffSpec::default(),
        ss,
        std::sync::Arc::new(theme),
    );
    app
}

#[test]
fn theme_preview_in_diff_mode_keeps_diff_lines() {
    let _guard = write_theme_test_state();
    let original = current_theme_selection();
    set_theme_preset(ThemePreset::OceanDark);
    let (ss, _) = test_assets();
    let ts = super::test_theme_set();
    let mut app = diff_app(SRC, None);
    let before = plain_lines(&app);

    app.open_theme_picker();
    app.preview_theme_preset(ThemePreset::Forest, &ss, ts);
    assert_eq!(
        plain_lines(&app),
        before,
        "preview must not re-render the diff"
    );

    app.restore_theme_picker_preview(&ss, ts);
    assert_eq!(
        plain_lines(&app),
        before,
        "escape must leave the diff intact"
    );
    assert!(!app.is_theme_picker_open());

    set_theme_selection(original);
}

#[test]
fn theme_preview_and_confirm_in_diff_mode_rebuild_diff_view() {
    let _guard = write_theme_test_state();
    let original = current_theme_selection();
    set_theme_preset(ThemePreset::OceanDark);
    let (ss, _) = test_assets();
    let ts = super::test_theme_set();
    let mut app = diff_app(SRC, None);
    let before = plain_lines(&app);

    app.open_theme_picker();
    app.preview_theme_preset(ThemePreset::Forest, &ss, ts);
    app.close_theme_picker();

    assert!(!app.is_theme_picker_open());
    assert!(app.is_diff_mode());
    assert_eq!(
        plain_lines(&app),
        before,
        "confirm must rebuild a diff, not Markdown"
    );
    assert!(
        plain_lines(&app).iter().any(|l| l.starts_with("┌─ a.rs")),
        "file frame must still be rendered"
    );

    set_theme_selection(original);
}

#[test]
fn rebuild_diff_view_keeps_current_file_and_layout() {
    let (ss, theme) = test_assets();
    let mut app = diff_app(SRC, None);
    app.build_split_now(&ss, &theme);
    app.diff_toggle_layout(&ss, &theme);
    assert!(app.diff_nav_to_file(1));

    app.rebuild_diff_view(&ss, &theme);

    let state = app.diff().unwrap();
    assert_eq!(state.layout, DiffLayout::Split);
    assert!(
        state.split_ready,
        "split layout must be rebuilt synchronously"
    );
    assert_eq!(app.diff_current_file_from_scroll(), Some(1));
    assert_eq!(Some(app.scroll()), state.first_scroll_target_of_file(1));
}

#[test]
fn rebuild_diff_view_resets_lazy_highlighting() {
    let (ss, theme) = test_assets();
    let mut app = diff_app(SRC, None);
    app.highlight_visible_diff_files(40);
    assert!(app.diff().unwrap().highlighted_files.iter().all(|&h| h));

    app.rebuild_diff_view(&ss, &theme);

    let state = app.diff().unwrap();
    assert!(state.highlighted_files.iter().all(|&h| !h));
    assert!(!state.split_ready, "the split view is rebuilt on demand");
}

#[test]
fn rebuild_diff_view_rebuilds_visible_preview() {
    let (ss, theme) = test_assets();
    let mut app = diff_app(SRC, None);
    assert!(app.diff_toggle_preview(&ss, &theme));

    app.rebuild_diff_view(&ss, &theme);

    let state = app.diff().unwrap();
    assert!(state.preview_visible);
    assert!(
        state.preview_cache.contains_key(&state.tree_active_idx),
        "the visible preview must not fall back to the placeholder"
    );
}

#[test]
fn reload_in_diff_mode_keeps_diff_view() {
    let (ss, _) = test_assets();
    let ts = super::test_theme_set();
    let dir = unique_temp_dir("leaf-diff-reload");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("change.patch");
    std::fs::write(&path, SRC).unwrap();
    let mut app = diff_app(SRC, Some(path.clone()));

    let updated = SRC.replace("+new", "+newer");
    std::fs::write(&path, &updated).unwrap();
    assert!(app.reload(&ss, ts));

    let lines = plain_lines(&app);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(app.is_diff_mode());
    assert!(lines.iter().any(|l| l.starts_with("┌─ a.rs")), "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("newer")), "{lines:?}");
}

#[test]
fn file_tree_on_a_narrow_terminal_does_not_panic() {
    use ratatui::{backend::TestBackend, Terminal};
    let mut app = diff_app(SRC, None);
    app.diff_toggle_tree();
    for width in [10, 30, 41] {
        let mut terminal = Terminal::new(TestBackend::new(width, 10)).unwrap();
        terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    }
}

#[test]
fn dragging_the_tree_border_on_a_narrow_terminal_does_not_panic() {
    let mut app = diff_app(SRC, None);
    app.diff_toggle_tree();
    app.terminal_width = 30;
    app.set_toc_width_cells(25);
    app.set_toc_width_cells(3);
}

#[test]
fn theme_preview_in_diff_mode_recolors_rows() {
    let _guard = write_theme_test_state();
    let original = current_theme_selection();
    set_theme_preset(ThemePreset::OceanDark);
    let (ss, _) = test_assets();
    let ts = super::test_theme_set();
    let mut app = diff_app(SRC, None);

    app.open_theme_picker();
    app.preview_theme_preset(ThemePreset::Forest, &ss, ts);

    let del_bg = crate::theme::app_theme().diff.del_bg;
    let del_row = app
        .visible_lines(0, app.total())
        .iter()
        .find(|l| {
            let text: String = l.spans.iter().map(|s| s.content.as_ref()).collect();
            text.contains("let a = 1")
        })
        .unwrap()
        .clone();
    assert!(
        del_row.spans.iter().any(|s| s.style.bg == Some(del_bg)),
        "the deleted row uses the previewed theme"
    );
    app.restore_theme_picker_preview(&ss, ts);
    set_theme_selection(original);
}

#[test]
fn lazy_highlighting_matches_an_eager_build() {
    let (ss, theme) = test_assets();
    let mut app = diff_app(SRC, None);
    app.highlight_visible_diff_files(usize::MAX / 2);
    let eager = crate::render::build_diff_lines(&parse_unified_diff(SRC), &ss, &theme);
    assert_eq!(app.visible_lines(0, app.total()), eager.lines.as_slice());

    app.build_split_now(&ss, &theme);
    assert!(app.diff_toggle_layout(&ss, &theme));
    app.highlight_visible_diff_files(usize::MAX / 2);
    let eager = crate::render::build_split_lines(&parse_unified_diff(SRC), &ss, &theme);
    assert_eq!(app.diff().unwrap().split_lines, eager.lines);
}

#[test]
fn long_changed_lines_skip_the_quadratic_intra_diff() {
    let (ss, theme) = test_assets();
    let old: String = (0..6000).map(|i| format!("a{i},")).collect();
    let new: String = (0..6000).map(|i| format!("b{i},")).collect();
    let src =
        format!("diff --git a/m.js b/m.js\n--- a/m.js\n+++ b/m.js\n@@ -1 +1 @@\n-{old}\n+{new}\n");
    let started = std::time::Instant::now();
    let built = crate::render::build_diff_lines(&parse_unified_diff(&src), &ss, &theme);
    assert!(built.lines.len() > 4);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn file_tree_renders_the_scrolled_window() {
    use ratatui::{backend::TestBackend, Terminal};
    let mut src = String::new();
    for i in 0..40 {
        src.push_str(&format!(
            "diff --git a/file{i:02}.txt b/file{i:02}.txt\n--- a/file{i:02}.txt\n+++ b/file{i:02}.txt\n@@ -1 +1 @@\n-a\n+b\n"
        ));
    }
    let mut app = diff_app(&src, None);
    app.diff_toggle_tree();
    let mut terminal = Terminal::new(TestBackend::new(100, 14)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    assert!(app.diff_nav_to_file(30));
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let screen: String = (0..14)
        .map(|y| {
            (0..100)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                + "\n"
        })
        .collect();
    assert!(screen.contains("file30.txt"), "{screen}");
    assert!(!screen.contains("file00.txt"), "{screen}");
}

#[test]
fn reload_in_split_layout_rebuilds_the_split_rows() {
    let (ss, theme) = test_assets();
    let ts = super::test_theme_set();
    let dir = unique_temp_dir("leaf-diff-reload-split");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("change.patch");
    std::fs::write(&path, SRC).unwrap();
    let mut app = diff_app(SRC, Some(path.clone()));
    app.build_split_now(&ss, &theme);
    assert!(app.diff_toggle_layout(&ss, &theme));

    std::fs::write(&path, SRC.replace("+new", "+newer")).unwrap();
    assert!(app.reload(&ss, ts));

    let _ = std::fs::remove_dir_all(&dir);
    let state = app.diff().unwrap();
    assert_eq!(state.layout, DiffLayout::Split);
    assert!(state.split_ready);
    assert!(!state.split_lines.is_empty());
}
