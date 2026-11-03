use super::{test_assets, test_md_theme, write_theme_test_state};
use crate::app::{App, AppConfig};
use crate::markdown::parse_markdown;
use crate::theme::{
    current_syntect_theme, current_theme_preset, current_theme_selection, resolve_theme_selection,
    set_theme_preset, set_theme_selection, theme_preset_index, validate_theme_syntax,
};
use crate::*;
use ratatui::style::Color;
use std::collections::BTreeMap;

#[test]
fn parse_theme_preset_supports_ocean_and_forest() {
    assert_eq!(parse_theme_preset("arctic"), Some(ThemePreset::Arctic));
    assert_eq!(parse_theme_preset("ocean"), Some(ThemePreset::OceanDark));
    assert_eq!(parse_theme_preset("forest"), Some(ThemePreset::Forest));
    assert_eq!(
        parse_theme_preset("solarized-dark"),
        Some(ThemePreset::SolarizedDark)
    );
    assert_eq!(parse_theme_preset("nope"), None);
}

#[test]
fn parse_theme_color_supports_hex_rgb_arrays_and_names() {
    assert_eq!(parse_theme_color("#112233"), Some(Color::Rgb(17, 34, 51)));
    assert_eq!(parse_theme_color("abc"), Some(Color::Rgb(170, 187, 204)));
    assert_eq!(parse_theme_color("rgb(1, 2, 3)"), Some(Color::Rgb(1, 2, 3)));
    assert_eq!(parse_theme_color("light-blue"), Some(Color::LightBlue));
    assert_eq!(parse_theme_color("not-a-color"), None);
}

#[test]
fn resolve_theme_selection_supports_custom_theme_overrides() {
    let custom: CustomThemeConfig = toml::from_str(
        r##"
base = "arctic"
syntax = "InspiredGitHub"

[ui]
content_bg = "#112233"

[markdown]
heading_1 = [1, 2, 3]
"##,
    )
    .unwrap();
    let mut themes = BTreeMap::new();
    themes.insert("midnight".to_string(), custom);

    let selection = resolve_theme_selection("midnight", &themes, None, None).unwrap();
    let ThemeSelection::Custom(custom) = selection else {
        panic!("expected custom theme selection");
    };

    assert_eq!(custom.name, "midnight");
    assert_eq!(custom.base_preset, ThemePreset::Arctic);
    assert_eq!(custom.theme.syntax_theme_name.as_ref(), "InspiredGitHub");
    assert_eq!(custom.theme.ui.content_bg, Color::Rgb(17, 34, 51));
    assert_eq!(custom.theme.markdown.heading_1, Color::Rgb(1, 2, 3));
    assert_eq!(
        custom.theme.markdown.heading_2,
        crate::theme::theme_by_preset(ThemePreset::Arctic)
            .markdown
            .heading_2
    );
}

#[test]
fn resolve_theme_selection_rejects_unknown_custom_base() {
    let custom: CustomThemeConfig = toml::from_str(r#"base = "missing""#).unwrap();
    let mut themes = BTreeMap::new();
    themes.insert("broken".to_string(), custom);

    let err = resolve_theme_selection("broken", &themes, None, None).unwrap_err();
    assert!(err.contains("Unknown base theme \"missing\""));
}

#[test]
fn validate_theme_syntax_warns_for_unknown_syntect_theme() {
    let custom: CustomThemeConfig = toml::from_str(r#"syntax = "missing-syntax-theme""#).unwrap();
    let mut themes = BTreeMap::new();
    themes.insert("custom".to_string(), custom);
    let selection = resolve_theme_selection("custom", &themes, None, None).unwrap();
    let theme_set = super::test_theme_set();

    let warning = validate_theme_syntax(&selection, theme_set).unwrap();
    assert!(warning.contains("missing-syntax-theme"));
}

#[test]
fn set_theme_selection_applies_custom_theme() {
    let _guard = write_theme_test_state();
    let original = current_theme_selection();
    let custom: CustomThemeConfig = toml::from_str(
        r##"[markdown]
text = "#010203"
"##,
    )
    .unwrap();
    let mut themes = BTreeMap::new();
    themes.insert("custom".to_string(), custom);
    let selection = resolve_theme_selection("custom", &themes, None, None).unwrap();

    set_theme_selection(selection);

    assert_eq!(app_theme().markdown.text, Color::Rgb(1, 2, 3));
    set_theme_selection(original);
}

#[test]
fn preset_themes_all_define_frame_fg() {
    for preset in THEME_PRESETS.iter().copied() {
        let theme = crate::theme::theme_by_preset(preset);

        match theme.diff.frame_fg {
            Color::Rgb(_, _, _) => {}
            other => panic!("preset {preset:?} frame_fg is not an RGB color: {other:?}"),
        }
    }
}

#[test]
fn custom_theme_overrides_frame_fg() {
    let custom: CustomThemeConfig = toml::from_str(
        r##"
base = "ocean"

[diff]
frame_fg = "#010203"
"##,
    )
    .unwrap();
    let mut themes = BTreeMap::new();
    themes.insert("custom".to_string(), custom);
    let selection = resolve_theme_selection("custom", &themes, None, None).unwrap();
    let ThemeSelection::Custom(custom) = selection else {
        panic!("expected custom theme selection");
    };
    assert_eq!(custom.theme.diff.frame_fg, Color::Rgb(1, 2, 3));
}

#[test]
fn theme_presets_are_in_alphabetical_order() {
    let labels: Vec<_> = THEME_PRESETS
        .iter()
        .map(|preset| theme_preset_label(*preset))
        .collect();
    let mut sorted = labels.clone();
    sorted.sort();
    assert_eq!(labels, sorted);
}

#[test]
fn theme_picker_restores_original_preset_on_escape() {
    let _guard = write_theme_test_state();
    let (ss, theme) = test_assets();
    let ts = super::test_theme_set();
    let (lines, toc, _, _) =
        parse_markdown("# Demo\n", &ss, &theme, &test_md_theme(), false, true).into();
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source: "# Demo\n".to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );

    let original = current_theme_selection();
    set_theme_preset(ThemePreset::OceanDark);
    app.open_theme_picker();
    assert!(app.set_theme_picker_index(theme_preset_index(ThemePreset::Forest)));
    app.preview_theme_preset(ThemePreset::Forest, &ss, ts);

    assert_eq!(current_theme_preset(), ThemePreset::Forest);

    app.restore_theme_picker_preview(&ss, ts);

    assert_eq!(current_theme_preset(), ThemePreset::OceanDark);
    assert!(!app.is_theme_picker_open());
    assert_eq!(app.theme_picker_original(), None);
    set_theme_selection(original);
}

#[test]
fn theme_picker_caches_previewed_themes_for_reuse() {
    let _guard = write_theme_test_state();
    let (ss, theme) = test_assets();
    let ts = super::test_theme_set();
    let (lines, toc, _, _) = parse_markdown(
        "# Demo\n\n```rs\nfn main() {}\n```\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source: "# Demo\n\n```rs\nfn main() {}\n```\n".to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );

    let original = current_theme_selection();
    set_theme_preset(ThemePreset::OceanDark);
    app.open_theme_picker();
    app.preview_theme_preset(ThemePreset::Forest, &ss, ts);

    assert!(app.has_cached_theme_preview(ThemePreset::Forest));
    assert_eq!(current_theme_preset(), ThemePreset::Forest);

    app.preview_theme_preset(ThemePreset::OceanDark, &ss, ts);
    assert_eq!(current_theme_preset(), ThemePreset::OceanDark);
    assert!(app.has_cached_theme_preview(ThemePreset::OceanDark));
    set_theme_selection(original);
}

#[test]
fn theme_picker_restores_custom_theme_on_escape() {
    let _guard = write_theme_test_state();
    let original = current_theme_selection();
    let custom: CustomThemeConfig = toml::from_str(
        r##"
base = "ocean"

[markdown]
heading_1 = "#010203"
"##,
    )
    .unwrap();
    let mut themes = BTreeMap::new();
    themes.insert("custom".to_string(), custom);
    let custom_selection = resolve_theme_selection("custom", &themes, None, None).unwrap();
    set_theme_selection(custom_selection.clone());

    let ss = super::test_syntax_set();
    let ts = super::test_theme_set();
    let theme = current_syntect_theme(ts).clone();
    let (lines, toc, _, _) =
        parse_markdown("# Demo\n", &ss, &theme, &test_md_theme(), false, true).into();
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source: "# Demo\n".to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );

    app.open_theme_picker();
    app.preview_theme_preset(ThemePreset::Forest, &ss, ts);
    assert_eq!(
        current_theme_selection().as_preset(),
        Some(ThemePreset::Forest)
    );

    app.restore_theme_picker_preview(&ss, ts);

    assert_eq!(current_theme_selection(), custom_selection);
    assert_eq!(app_theme().markdown.heading_1, Color::Rgb(1, 2, 3));
    set_theme_selection(original);
}

#[test]
fn theme_picker_cached_previews_keep_full_parse_metadata() {
    let _guard = write_theme_test_state();
    let original = current_theme_selection();
    set_theme_preset(ThemePreset::Forest);
    let source = "[A](https://a.test)\n\n```rust\nlet x = 1;\n```\n";
    let (ss, _) = test_assets();
    let ts = super::test_theme_set();
    let mut app = App::new_with_source(
        Vec::new(),
        Vec::new(),
        AppConfig {
            filename: "stdin".to_string(),
            source: source.to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );
    assert!(app.sync_render_width(60, &ss, ts));
    let metadata = |app: &App| {
        (
            app.code_blocks.len(),
            app.line_number_total(),
            app.source_line_at(2),
            app.link_spans_by_line
                .get(&0)
                .and_then(|spans| spans.first())
                .and_then(|span| app.link_url(span.link_id))
                .map(str::to_owned),
        )
    };
    let state = |app: &App| (app.visible_lines(0, app.total()).to_vec(), metadata(app));
    let before = state(&app);
    let (code_blocks, line_total, source_line, link) = &before.1;
    assert_eq!(*code_blocks, 1);
    assert!(*line_total > 0);
    assert_eq!(*source_line, 3);
    assert_eq!(link.as_deref(), Some("https://a.test"));

    app.open_theme_picker();
    app.preview_theme_preset(ThemePreset::OceanDark, &ss, ts);
    let ocean = state(&app);
    assert_ne!(ocean.0, before.0);
    assert_eq!(ocean.1, before.1);

    app.preview_theme_preset(ThemePreset::Forest, &ss, ts);
    assert_eq!(state(&app), before, "cached original preset");
    app.preview_theme_preset(ThemePreset::OceanDark, &ss, ts);
    assert_eq!(state(&app), ocean, "cached previewed preset");

    app.restore_theme_picker_preview(&ss, ts);
    assert_eq!(state(&app), before, "escape restore");
    app.content_area = ratatui::layout::Rect::new(0, 0, 60, 10);
    app.enter_code_select_mode();
    assert_eq!(app.highlighted_code_block(), Some(0));
    set_theme_selection(original);
}

#[test]
fn resizing_during_theme_preview_reparses_original_theme_at_new_width() {
    let _guard = write_theme_test_state();
    let original = current_theme_selection();
    set_theme_preset(ThemePreset::Forest);
    let source = "This paragraph is intentionally long so its rendered line count changes when the terminal is resized to a much narrower width.\n\n[A](https://a.test)\n\n```rust\nlet value = \"this code line also wraps when the available width is reduced\";\n```\n";
    let (ss, _) = test_assets();
    let ts = super::test_theme_set();
    let mut app = App::new_with_source(
        Vec::new(),
        Vec::new(),
        AppConfig {
            filename: "stdin".to_string(),
            source: source.to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );
    assert!(app.sync_render_width(60, &ss, ts));
    let wide_total = app.total();

    app.open_theme_picker();
    app.preview_theme_preset(ThemePreset::OceanDark, &ss, ts);
    assert!(app.sync_render_width(24, &ss, ts));
    let narrow_total = app.total();
    assert!(
        narrow_total > wide_total,
        "fixture must reflow at narrow width"
    );

    app.restore_theme_picker_preview(&ss, ts);
    assert_eq!(app.total(), narrow_total);
    assert_eq!(app.code_blocks.len(), 1);
    assert_eq!(
        app.link_spans_by_line
            .values()
            .flatten()
            .find_map(|span| app.link_url(span.link_id)),
        Some("https://a.test")
    );
    set_theme_selection(original);
}
