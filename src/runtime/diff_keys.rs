use crate::app::App;
#[cfg(test)]
use crossterm::event::KeyModifiers;
use crossterm::event::{KeyCode, KeyEvent};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

pub(super) fn handle_diff_keys(
    app: &mut App,
    key: &KeyEvent,
    ss: &SyntaxSet,
    themes: &ThemeSet,
) -> bool {
    match key.code {
        KeyCode::Char('n') => {
            if app.has_active_search() {
                app.next_match();
                return true;
            }
            if app.is_diff_preview_visible() {
                let theme = crate::theme::current_syntect_theme(themes);
                app.diff_preview_step_file(1, ss, theme);
            } else {
                app.diff_next_file();
            }
            true
        }
        KeyCode::Char('N') => {
            if app.has_active_search() {
                app.prev_match();
                return true;
            }
            if app.is_diff_preview_visible() {
                let theme = crate::theme::current_syntect_theme(themes);
                app.diff_preview_step_file(-1, ss, theme);
            } else {
                app.diff_prev_file();
            }
            true
        }
        KeyCode::Char('t') => {
            app.diff_toggle_tree();
            true
        }
        KeyCode::Char('v') => {
            let theme = crate::theme::current_syntect_theme(themes);
            app.diff_toggle_layout(ss, theme);
            true
        }
        KeyCode::Char('p') => {
            let theme = crate::theme::current_syntect_theme(themes);
            app.diff_toggle_preview(ss, theme);
            true
        }

        KeyCode::Char('r') | KeyCode::Char(' ') => {
            if !app.is_diff_preview_visible() {
                app.diff_recenter_on_current_file();
            }
            true
        }

        KeyCode::Enter => {
            if !app.is_diff_preview_visible() {
                let theme = crate::theme::current_syntect_theme(themes);
                app.diff_toggle_preview(ss, theme);
            }
            true
        }

        KeyCode::Char('J') if app.can_scroll_diff_tree() => {
            app.focus_next_diff_tree_file();
            true
        }
        KeyCode::Char('K') if app.can_scroll_diff_tree() => {
            app.focus_prev_diff_tree_file();
            true
        }

        KeyCode::Char('D') if app.can_scroll_diff_tree() => {
            app.focus_next_diff_tree_file_by(5);
            true
        }
        KeyCode::Char('U') if app.can_scroll_diff_tree() => {
            app.focus_prev_diff_tree_file_by(5);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppMode, DiffSpec};
    use crate::diff::parse_unified_diff;
    use ratatui::text::Line;
    use syntect::highlighting::ThemeSet;
    use syntect::parsing::SyntaxSet;

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

    fn syntect_defaults() -> (SyntaxSet, syntect::highlighting::Theme, ThemeSet) {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let theme = ts.themes["base16-ocean.dark"].clone();
        (ss, theme, ts)
    }

    #[test]
    fn shift_j_in_diff_mode_moves_tree_cursor_down() {
        let src = concat!(
            "diff --git a/a.md b/a.md\n",
            "--- a/a.md\n+++ b/a.md\n@@ -1 +1 @@\n-x\n+y\n",
            "diff --git a/b.md b/b.md\n",
            "--- a/b.md\n+++ b/b.md\n@@ -1 +1 @@\n-x\n+y\n",
            "diff --git a/c.md b/c.md\n",
            "--- a/c.md\n+++ b/c.md\n@@ -1 +1 @@\n-x\n+y\n",
        );
        let files = parse_unified_diff(src);
        let mut app = make_app();
        let (ss, theme, themes) = syntect_defaults();
        app.set_source_for_test(src.to_string());
        app.install_diff(
            files,
            DiffSpec::default(),
            std::sync::Arc::new(ss.clone()),
            std::sync::Arc::new(theme.clone()),
        );

        app.diff_toggle_tree();
        app.diff_tree_list_area = Some(ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 10,
        });
        assert!(app.can_scroll_diff_tree());
        assert_eq!(app.diff().unwrap().tree_active_idx, 0);

        let key = KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE);
        let handled = handle_diff_keys(&mut app, &key, &ss, &themes);
        assert!(handled, "Shift+J must be handled when the tree is open");
        assert_eq!(app.diff().unwrap().tree_active_idx, 1);
        assert_eq!(app.diff().unwrap().current_file_idx, 1);
    }

    #[test]
    fn p_in_diff_mode_toggles_preview_not_path_popup() {
        let src = concat!(
            "diff --git a/x.md b/x.md\n",
            "--- a/x.md\n",
            "+++ b/x.md\n",
            "@@ -1 +1 @@\n",
            "-old\n",
            "+new\n",
        );
        let files = parse_unified_diff(src);
        let mut app = make_app();
        let (ss, theme, themes) = syntect_defaults();
        app.set_source_for_test(src.to_string());
        app.install_diff(
            files,
            DiffSpec::default(),
            std::sync::Arc::new(ss.clone()),
            std::sync::Arc::new(theme.clone()),
        );
        assert_eq!(app.mode(), AppMode::Diff);
        assert!(!app.is_path_popup_open());
        assert!(!app.diff().unwrap().preview_visible);

        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
        let handled = handle_diff_keys(&mut app, &key, &ss, &themes);
        assert!(handled);

        assert!(app.diff().unwrap().preview_visible);
        assert!(!app.is_path_popup_open());
    }

    #[test]
    fn p_in_document_mode_opens_path_popup() {
        let mut app = make_app();
        assert_eq!(app.mode(), AppMode::Document);
        assert!(!app.is_path_popup_open());

        if app.mode() == AppMode::Document {
            app.open_path_popup();
        }
        assert!(app.is_path_popup_open());
    }

    #[test]
    fn space_in_diff_mode_recenters_on_current_file() {
        let src = concat!(
            "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-x\n+y\n",
            "diff --git a/b b/b\n--- a/b\n+++ b/b\n@@ -1 +1 @@\n-x\n+y\n",
            "diff --git a/c b/c\n--- a/c\n+++ b/c\n@@ -1 +1 @@\n-x\n+y\n",
        );
        let files = parse_unified_diff(src);
        let mut app = make_app();
        let (ss, theme, themes) = syntect_defaults();
        app.set_source_for_test(src.to_string());
        app.install_diff(
            files,
            DiffSpec::default(),
            std::sync::Arc::new(ss.clone()),
            std::sync::Arc::new(theme.clone()),
        );
        app.diff_next_file();
        let expected = app
            .diff()
            .and_then(|s| s.first_scroll_target_of_file(s.current_file_idx))
            .expect("second file has a top-frame row");
        app.scroll_down(5);
        assert_ne!(app.scroll(), expected);

        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        assert!(handle_diff_keys(&mut app, &key, &ss, &themes));
        assert_eq!(app.scroll(), expected);
    }

    #[test]
    fn r_in_diff_mode_recenters_like_space() {
        let src = concat!(
            "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-x\n+y\n",
            "diff --git a/b b/b\n--- a/b\n+++ b/b\n@@ -1 +1 @@\n-x\n+y\n",
        );
        let files = parse_unified_diff(src);
        let mut app = make_app();
        let (ss, theme, themes) = syntect_defaults();
        app.set_source_for_test(src.to_string());
        app.install_diff(
            files,
            DiffSpec::default(),
            std::sync::Arc::new(ss.clone()),
            std::sync::Arc::new(theme.clone()),
        );
        app.diff_next_file();
        let expected = app
            .diff()
            .and_then(|s| s.first_scroll_target_of_file(s.current_file_idx))
            .expect("second file has a top-frame row");
        app.scroll_down(5);
        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
        assert!(handle_diff_keys(&mut app, &key, &ss, &themes));
        assert_eq!(app.scroll(), expected);
    }

    #[test]
    fn space_in_preview_mode_is_noop() {
        let src = "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-x\n+y\n";
        let files = parse_unified_diff(src);
        let mut app = make_app();
        let (ss, theme, themes) = syntect_defaults();
        app.set_source_for_test(src.to_string());
        app.install_diff(
            files,
            DiffSpec::default(),
            std::sync::Arc::new(ss.clone()),
            std::sync::Arc::new(theme.clone()),
        );
        let open_preview = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
        handle_diff_keys(&mut app, &open_preview, &ss, &themes);
        assert!(app.diff().unwrap().preview_visible);
        let before_scroll = app.scroll();

        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        assert!(handle_diff_keys(&mut app, &key, &ss, &themes));
        assert_eq!(app.scroll(), before_scroll);
        assert!(app.diff().unwrap().preview_visible);
    }

    #[test]
    fn enter_in_diff_mode_opens_preview() {
        let src = "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-x\n+y\n";
        let files = parse_unified_diff(src);
        let mut app = make_app();
        let (ss, theme, themes) = syntect_defaults();
        app.set_source_for_test(src.to_string());
        app.install_diff(
            files,
            DiffSpec::default(),
            std::sync::Arc::new(ss.clone()),
            std::sync::Arc::new(theme.clone()),
        );
        assert!(!app.diff().unwrap().preview_visible);

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert!(handle_diff_keys(&mut app, &key, &ss, &themes));
        assert!(app.diff().unwrap().preview_visible);
    }

    #[test]
    fn enter_in_preview_mode_is_noop() {
        let src = "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-x\n+y\n";
        let files = parse_unified_diff(src);
        let mut app = make_app();
        let (ss, theme, themes) = syntect_defaults();
        app.set_source_for_test(src.to_string());
        app.install_diff(
            files,
            DiffSpec::default(),
            std::sync::Arc::new(ss.clone()),
            std::sync::Arc::new(theme.clone()),
        );
        let open_preview = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
        handle_diff_keys(&mut app, &open_preview, &ss, &themes);
        assert!(app.diff().unwrap().preview_visible);

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert!(handle_diff_keys(&mut app, &key, &ss, &themes));
        assert!(
            app.diff().unwrap().preview_visible,
            "Enter in preview mode must NOT toggle the preview back off"
        );
    }

    #[test]
    fn tab_in_diff_mode_is_not_handled() {
        let src = "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-x\n+y\n";
        let files = parse_unified_diff(src);
        let mut app = make_app();
        let (ss, theme, themes) = syntect_defaults();
        app.set_source_for_test(src.to_string());
        app.install_diff(
            files,
            DiffSpec::default(),
            std::sync::Arc::new(ss.clone()),
            std::sync::Arc::new(theme.clone()),
        );

        let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let handled = handle_diff_keys(&mut app, &key, &ss, &themes);
        assert!(!handled, "Tab must no longer be bound in Diff mode");
    }

    #[test]
    fn n_in_diff_mode_with_active_search_steps_match() {
        let src = concat!(
            "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-x\n+hello\n",
            "diff --git a/b b/b\n--- a/b\n+++ b/b\n@@ -1 +1 @@\n-x\n+hello\n",
        );
        let files = parse_unified_diff(src);
        let mut app = make_app();
        let (ss, theme, themes) = syntect_defaults();
        app.set_source_for_test(src.to_string());
        app.install_diff(
            files,
            DiffSpec::default(),
            std::sync::Arc::new(ss.clone()),
            std::sync::Arc::new(theme.clone()),
        );
        app.set_search_query("hello");
        app.run_search();
        assert!(app.has_active_search());
        assert!(app.search_matches().len() >= 2);
        let file_before = app.diff().unwrap().current_file_idx;

        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        assert!(handle_diff_keys(&mut app, &key, &ss, &themes));
        assert_eq!(
            app.diff().unwrap().current_file_idx,
            file_before,
            "n with active search must step the match cursor, not the file cursor"
        );
    }

    #[test]
    fn n_in_diff_mode_without_search_still_steps_file() {
        let src = concat!(
            "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-x\n+y\n",
            "diff --git a/b b/b\n--- a/b\n+++ b/b\n@@ -1 +1 @@\n-x\n+y\n",
        );
        let files = parse_unified_diff(src);
        let mut app = make_app();
        let (ss, theme, themes) = syntect_defaults();
        app.set_source_for_test(src.to_string());
        app.install_diff(
            files,
            DiffSpec::default(),
            std::sync::Arc::new(ss.clone()),
            std::sync::Arc::new(theme.clone()),
        );
        assert!(!app.has_active_search());

        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        assert!(handle_diff_keys(&mut app, &key, &ss, &themes));
        assert_eq!(app.diff().unwrap().current_file_idx, 1);
    }

    #[test]
    fn p_guard_gates_open_path_popup_by_mode() {
        let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
        let files = parse_unified_diff(src);
        let mut app = make_app();
        let (ss, theme, _themes) = syntect_defaults();
        app.install_diff(
            files,
            DiffSpec::default(),
            std::sync::Arc::new(ss.clone()),
            std::sync::Arc::new(theme.clone()),
        );
        assert_eq!(app.mode(), AppMode::Diff);

        if app.mode() == AppMode::Document {
            app.open_path_popup();
        }
        assert!(!app.is_path_popup_open());
    }
}
