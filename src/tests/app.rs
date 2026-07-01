use super::{test_assets, test_md_theme};
use crate::app::{App, AppConfig, FileChange};
use crate::cli::parse_cli;
use crate::markdown::{hash_str, parse_markdown, parse_markdown_with_width, read_file_state};
use crate::*;
use crossterm::event::KeyEventKind;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};
use syntect::highlighting::ThemeSet;

#[test]
fn search_matches_across_span_boundaries() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) = parse_markdown(
        "hello **world**",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("hello world");
    app.run_search();

    assert_eq!(app.search_match_count(), 1);
    assert!(line_plain_text(app.line(app.search_matches()[0]).unwrap()).contains("hello world"));
}

#[test]
fn key_release_events_are_ignored() {
    assert!(should_handle_key(KeyEventKind::Press));
    assert!(should_handle_key(KeyEventKind::Repeat));
    assert!(!should_handle_key(KeyEventKind::Release));
}

#[test]
fn mouse_capture_is_enabled_by_default_and_toggles() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) =
        parse_markdown("body", &ss, &theme, &test_md_theme(), false, true).into();
    let mut app = App::new(lines, toc, "test".to_string(), false, false, None, None);
    assert!(app.is_mouse_capture_enabled());
    assert!(!app.toggle_mouse_capture());
    assert!(!app.is_mouse_capture_enabled());
    assert!(app.toggle_mouse_capture());
    assert!(app.is_mouse_capture_enabled());
}

#[test]
fn toggle_mouse_capture_off_resets_hover_and_drag_state() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) =
        parse_markdown("body", &ss, &theme, &test_md_theme(), false, true).into();
    let mut app = App::new(lines, toc, "test".to_string(), false, false, None, None);
    app.hovered_link = Some((3, 0));
    app.hovered_toc_idx = Some(2);
    app.scrollbar_dragging = true;
    app.toggle_mouse_capture();
    assert_eq!(app.hovered_link, None);
    assert_eq!(app.hovered_toc_idx, None);
    assert!(!app.scrollbar_dragging);
}

#[test]
fn stdin_read_is_rejected_when_over_limit() {
    let mut cursor = std::io::Cursor::new(vec![b'a'; 5]);
    let err = read_stdin_with_limit(&mut cursor, 4).unwrap_err();
    assert!(err
        .to_string()
        .contains("stdin exceeds the maximum supported size"));
}

#[test]
fn parse_cli_accepts_update_on_its_own() {
    let args = vec!["leaf".to_string(), "--update".to_string()];
    let options = parse_cli(&args).unwrap();

    assert!(options.update);
    assert!(!options.watch);
    assert_eq!(options.file_arg, None);
}

#[test]
fn parse_cli_rejects_update_with_other_flags() {
    let args = vec![
        "leaf".to_string(),
        "--update".to_string(),
        "--watch".to_string(),
    ];

    let err = parse_cli(&args).unwrap_err();
    assert!(err.to_string().contains("--update must be used on its own"));
}

#[test]
fn parse_cli_accepts_config_on_its_own() {
    let args = vec!["leaf".to_string(), "--config".to_string()];
    let options = parse_cli(&args).unwrap();

    assert_eq!(options.config, Some(cli::ConfigAction::Open));
    assert!(!options.update);
    assert!(!options.watch);
    assert_eq!(options.file_arg, None);
}

#[test]
fn parse_cli_rejects_config_with_other_flags() {
    let args = vec![
        "leaf".to_string(),
        "--config".to_string(),
        "--watch".to_string(),
    ];

    let err = parse_cli(&args).unwrap_err();
    assert!(err.to_string().contains("--config must be used on its own"));
}

#[test]
fn parse_cli_rejects_update_with_config() {
    let args = vec![
        "leaf".to_string(),
        "--update".to_string(),
        "--config".to_string(),
    ];

    let err = parse_cli(&args).unwrap_err();
    assert!(err.to_string().contains("must be used on its own"));
}

#[test]
fn parse_cli_config_reset() {
    let args = vec![
        "leaf".to_string(),
        "--config".to_string(),
        "reset".to_string(),
    ];
    let options = parse_cli(&args).unwrap();
    assert_eq!(options.config, Some(cli::ConfigAction::Reset));
}

#[test]
fn parse_cli_rejects_config_reset_with_other_flags() {
    let args = vec![
        "leaf".to_string(),
        "--config".to_string(),
        "reset".to_string(),
        "--watch".to_string(),
    ];
    let err = parse_cli(&args).unwrap_err();
    assert!(err.to_string().contains("--config must be used on its own"));
}

#[test]
fn parse_cli_accepts_picker_on_its_own() {
    let args = vec!["leaf".to_string(), "--picker".to_string()];
    let options = parse_cli(&args).unwrap();

    assert!(options.picker);
    assert!(!options.watch);
    assert_eq!(options.file_arg, None);
}

#[test]
fn parse_cli_accepts_picker_with_watch() {
    let args = vec![
        "leaf".to_string(),
        "--picker".to_string(),
        "--watch".to_string(),
    ];

    let options = parse_cli(&args).unwrap();
    assert!(options.picker);
    assert!(options.watch);
    assert_eq!(options.file_arg, None);
}

#[test]
fn parse_cli_accepts_custom_theme_names() {
    let args = vec![
        "leaf".to_string(),
        "--theme".to_string(),
        "my-theme".to_string(),
        "README.md".to_string(),
    ];

    let options = parse_cli(&args).unwrap();
    assert_eq!(options.theme.as_deref(), Some("my-theme"));
    assert_eq!(options.file_arg.as_deref(), Some("README.md"));
}

#[test]
fn parse_cli_rejects_empty_theme_name() {
    let args = vec!["leaf".to_string(), "--theme=".to_string()];

    let err = parse_cli(&args).unwrap_err();
    assert!(err.to_string().contains("Missing value for --theme"));
}

#[test]
fn cancelling_search_clears_query_and_matches() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) = parse_markdown(
        "alpha\nbeta\nalpha beta\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();

    app.begin_search();
    app.set_search_draft("alpha gamma");
    app.cancel_search();

    assert!(!app.is_search_mode());
    assert!(app.search_draft().is_empty());
    assert!(app.search_query().is_empty());
    assert!(app.search_matches().is_empty());
    assert_eq!(app.search_index(), 0);
}

#[test]
fn confirm_search_uses_draft_and_updates_matches() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) = parse_markdown(
        "alpha\nbeta\nbeta\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.begin_search();
    app.set_search_draft("beta");
    app.confirm_search();

    assert!(!app.is_search_mode());
    assert!(app.search_draft().is_empty());
    assert_eq!(app.search_query(), "beta");
    assert_eq!(app.search_match_count(), 2);
}

#[test]
fn confirm_search_with_new_query_restarts_from_first_match() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) = parse_markdown(
        "alpha\nbeta\nbeta again\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();

    app.begin_search();
    app.set_search_draft("beta");
    app.confirm_search();

    assert_eq!(app.search_query(), "beta");
    assert_eq!(app.search_index(), 0);
    assert_eq!(app.scroll(), app.search_matches()[0]);
    assert_eq!(app.search_match_count(), 2);
}

#[test]
fn enter_in_normal_mode_advances_active_search() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) = parse_markdown(
        "alpha\nbeta alpha\nalpha again\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();
    let second_match = app.search_matches()[1];

    app.next_match();

    assert_eq!(app.search_index(), 1);
    assert_eq!(app.scroll(), second_match);
}

#[test]
fn ctrl_c_cancels_search_prompt_and_clears_active_query() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) =
        parse_markdown("alpha\nbeta\n", &ss, &theme, &test_md_theme(), false, true).into();
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();

    app.begin_search();
    app.push_search_draft('z');
    app.cancel_search();

    assert!(!app.is_search_mode());
    assert!(app.search_query().is_empty());
    assert!(app.search_matches().is_empty());
    assert_eq!(app.search_index(), 0);
}

#[test]
fn esc_clears_active_search_from_normal_mode() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) = parse_markdown(
        "alpha\nbeta alpha\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();
    app.clear_active_search();

    assert!(!app.is_search_mode());
    assert!(app.search_draft().is_empty());
    assert!(app.search_query().is_empty());
    assert!(app.search_matches().is_empty());
    assert_eq!(app.search_index(), 0);
}

#[test]
fn ctrl_c_clears_active_search_before_exit() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) = parse_markdown(
        "alpha\nbeta alpha\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let mut app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    app.set_search_query("alpha");
    app.run_search();
    app.clear_active_search();

    assert!(!app.has_active_search());
    assert!(app.search_query().is_empty());
    assert!(app.search_matches().is_empty());
}

#[test]
fn active_highlight_line_is_none_without_search_matches() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) =
        parse_markdown("alpha\nbeta\n", &ss, &theme, &test_md_theme(), false, true).into();
    let app = App::new(lines, toc, "stdin".to_string(), false, false, None, None);

    assert_eq!(app.active_highlight_line(), None);
}

#[test]
fn check_modified_detects_file_metadata_change() {
    let (ss, theme) = test_assets();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("leaf-check-modified-{unique}.md"));
    fs::write(&path, "# Before\n").unwrap();

    let src = fs::read_to_string(&path).unwrap();
    let (lines, toc, _, _) =
        parse_markdown(&src, &ss, &theme, &test_md_theme(), false, true).into();
    let state = read_file_state(&path).unwrap();
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: path.file_name().unwrap().to_string_lossy().to_string(),
            source: src.clone(),
            debug_input: false,
            watch: true,
            filepath: Some(path.clone()),
            last_file_state: Some(state),
        },
    );
    app.set_last_content_hash(hash_str(&src));

    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(&path, "# After\nextra\n").unwrap();

    let change = app.check_modified();
    assert!(matches!(
        change,
        Some(FileChange::Metadata(_)) | Some(FileChange::Content(_))
    ));

    let _ = fs::remove_file(path);
}

#[test]
fn reload_returns_false_when_file_cannot_be_read() {
    let (ss, _theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("leaf-reload-fail-{unique}.md"));
    fs::write(&path, "# Demo\n").unwrap();

    let mut app = App::new_with_source(
        Vec::new(),
        Vec::new(),
        AppConfig {
            filename: "picker".to_string(),
            source: String::new(),
            debug_input: false,
            watch: true,
            filepath: None,
            last_file_state: None,
        },
    );
    assert!(app.load_path(path.clone(), &ss, &ts));

    fs::remove_file(&path).unwrap();
    assert!(!app.reload(&ss, &ts));
}

#[test]
fn sync_render_width_preserves_scroll_proportion() {
    let (ss, theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let source = (0..12)
        .map(|idx| {
            format!(
                "Paragraph {idx} has enough repeated content to wrap differently when the render width changes significantly across reparses."
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let (lines, toc, _, _) =
        parse_markdown_with_width(&source, &ss, &theme, 80, &test_md_theme(), false, true).into();
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source,
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );

    app.scroll_down(8);
    let old_scroll = app.scroll();
    let old_total = app.total();
    assert!(app.sync_render_width(24, &ss, &ts));

    let new_total = app.total();
    let expected = ((old_scroll as f64 / old_total as f64) * new_total as f64) as usize;
    assert_eq!(app.scroll(), expected.min(new_total.saturating_sub(1)));
}

#[test]
fn check_modified_reports_metadata_when_no_previous_file_state() {
    let (ss, theme) = test_assets();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("leaf-check-modified-initial-{unique}.md"));
    fs::write(&path, "# Initial\n").unwrap();

    let src = fs::read_to_string(&path).unwrap();
    let (lines, toc, _, _) =
        parse_markdown(&src, &ss, &theme, &test_md_theme(), false, true).into();
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: path.file_name().unwrap().to_string_lossy().to_string(),
            source: src.clone(),
            debug_input: false,
            watch: true,
            filepath: Some(path.clone()),
            last_file_state: None,
        },
    );
    app.set_last_content_hash(hash_str(&src));

    assert!(matches!(
        app.check_modified(),
        Some(FileChange::Metadata(_))
    ));

    let _ = fs::remove_file(path);
}

#[test]
fn sync_render_width_returns_false_when_clamped_width_is_unchanged() {
    let (ss, theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let source = "One paragraph that does not matter much for this width clamp test.";
    let (lines, toc, _, _) =
        parse_markdown_with_width(source, &ss, &theme, 20, &test_md_theme(), false, true).into();
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source: source.to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );

    assert!(app.sync_render_width(10, &ss, &ts));
    assert!(!app.sync_render_width(10, &ss, &ts));
    assert_eq!(
        app.total(),
        parse_markdown_with_width(source, &ss, &theme, 20, &test_md_theme(), false, true)
            .lines
            .len()
    );
}

#[test]
fn initial_mode_has_no_content() {
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) = parse_markdown("", &ss, &theme, &test_md_theme(), false, true).into();
    let app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "test".to_string(),
            source: String::new(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );
    assert!(!app.has_content(), "initial mode should have no content");
}

#[test]
fn preview_mode_has_content() {
    let src = "# Hello";
    let (ss, theme) = test_assets();
    let (lines, toc, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "test".to_string(),
            source: src.to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );
    assert!(
        app.has_content(),
        "preview mode with source should have content"
    );
}

#[test]
fn load_path_activates_watch_from_config() {
    let (ss, _theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("leaf-watch-config-{unique}.md"));
    fs::write(&path, "# Demo\n").unwrap();

    let mut app = App::new_with_source(
        Vec::new(),
        Vec::new(),
        AppConfig {
            filename: "picker".to_string(),
            source: String::new(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );
    app.set_watch_from_config(true);

    assert!(!app.is_watch_enabled());
    assert!(app.load_path(path.clone(), &ss, &ts));
    assert!(app.is_watch_enabled());

    // Toggle watch off, then load another file — watch stays off
    app.toggle_watch();
    assert!(!app.is_watch_enabled());

    let path2 = std::env::temp_dir().join(format!("leaf-watch-config2-{unique}.md"));
    fs::write(&path2, "# Second\n").unwrap();
    assert!(app.load_path(path2.clone(), &ss, &ts));
    assert!(!app.is_watch_enabled());

    let _ = fs::remove_file(path);
    let _ = fs::remove_file(path2);
}
