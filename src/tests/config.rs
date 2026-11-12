use crate::config::{parse_page_scroll_size, PageScrollSize, PageScrollSizeSetting};
use crate::*;

#[test]
fn parse_full_config() {
    let toml = r#"
theme = "forest"
editor = "vim"
watch = true
extras = ["txt", "csv"]
"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.theme.as_deref(), Some("forest"));
    assert_eq!(config.editor.as_deref(), Some("vim"));
    assert_eq!(config.watch, Some(true));
    assert_eq!(config.extras, vec!["txt", "csv"]);
    assert!(config.themes.is_empty());
}

#[test]
fn parse_partial_config_theme_only() {
    let toml = r#"theme = "arctic""#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.theme.as_deref(), Some("arctic"));
    assert_eq!(config.editor, None);
    assert_eq!(config.watch, None);
    assert!(config.themes.is_empty());
}

#[test]
fn parse_empty_config() {
    let toml = "";
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.theme, None);
    assert_eq!(config.editor, None);
    assert_eq!(config.watch, None);
    assert!(config.extras.is_empty());
    assert!(config.themes.is_empty());
}

#[test]
fn parse_invalid_toml_returns_default() {
    let toml = "not valid {{{{ toml";
    let config: Result<LeafConfig, _> = toml::from_str(toml);
    assert!(config.is_err());
    let fallback = config.unwrap_or_default();
    assert_eq!(fallback.theme, None);
    assert_eq!(fallback.editor, None);
    assert_eq!(fallback.watch, None);
    assert!(fallback.extras.is_empty());
    assert!(fallback.themes.is_empty());
}

#[test]
fn parse_extras_config() {
    let toml = r#"extras = ["txt", "csv", "log"]"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.extras, vec!["txt", "csv", "log"]);
}

#[test]
fn parse_extras_empty_array() {
    let toml = r#"extras = []"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert!(config.extras.is_empty());
}

#[test]
fn parse_extras_missing_defaults_to_empty() {
    let toml = r#"theme = "ocean""#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert!(config.extras.is_empty());
}

#[test]
fn unknown_fields_are_ignored() {
    let toml = r#"
theme = "ocean"
unknown_field = 42
"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.theme.as_deref(), Some("ocean"));
}

#[test]
fn invalid_theme_is_not_a_known_preset() {
    let toml = r#"theme = "nonexistent""#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.theme.as_deref(), Some("nonexistent"));
    assert!(parse_theme_preset("nonexistent").is_none());
}

#[test]
fn parse_custom_theme_config() {
    let toml = r##"
theme = "my-theme"

[themes.my-theme]
base = "forest"
syntax = "base16-ocean.dark"

[themes.my-theme.ui]
content_bg = "#101010"

[themes.my-theme.markdown]
text = [220, 221, 222]
"##;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    let selection = crate::theme::resolve_theme_selection(
        config.theme.as_deref().unwrap(),
        &config.themes,
        None,
        None,
    )
    .unwrap();

    let ThemeSelection::Custom(custom) = selection else {
        panic!("expected custom theme");
    };

    assert_eq!(custom.name, "my-theme");
    assert_eq!(custom.base_preset, ThemePreset::Forest);
    assert_eq!(
        custom.theme.ui.content_bg,
        ratatui::style::Color::Rgb(16, 16, 16)
    );
    assert_eq!(
        custom.theme.markdown.text,
        ratatui::style::Color::Rgb(220, 221, 222)
    );
}

#[test]
fn repository_config_keeps_ocean_default() {
    let config: LeafConfig = toml::from_str(include_str!("../../config.toml")).unwrap();
    assert_eq!(config.theme.as_deref(), Some("ocean"));
    assert!(config.themes.is_empty());
}

#[test]
fn external_theme_file_resolves_relative_to_config_dir() {
    let dir = super::unique_temp_dir("leaf-theme-config");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("gruvbox.toml"),
        r##"
base = "ocean"
syntax = "base16-ocean.dark"

[ui]
content_bg = "#282828"

[markdown]
text = "#ebdbb2"
heading_1 = "#fabd2f"
"##,
    )
    .unwrap();

    let themes = std::collections::BTreeMap::new();
    let selection =
        crate::theme::resolve_theme_selection("gruvbox.toml", &themes, Some(&dir), None).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();

    let ThemeSelection::Custom(custom) = selection else {
        panic!("expected gruvbox to resolve as a custom theme");
    };

    assert_eq!(custom.name, "gruvbox");
    assert_eq!(custom.base_preset, ThemePreset::OceanDark);
    assert_eq!(
        custom.theme.ui.content_bg,
        ratatui::style::Color::Rgb(40, 40, 40)
    );
    assert_eq!(
        custom.theme.markdown.text,
        ratatui::style::Color::Rgb(235, 219, 178)
    );
    assert_eq!(
        custom.theme.markdown.heading_1,
        ratatui::style::Color::Rgb(250, 189, 47)
    );
}

#[test]
fn theme_name_resolves_from_themes_dir() {
    let dir = super::unique_temp_dir("leaf-themes-dir");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("rose-pine-moon.toml"),
        "base = \"ocean\"\n\n[ui]\ncontent_bg = \"#232136\"\n",
    )
    .unwrap();

    let themes = std::collections::BTreeMap::new();
    let selection =
        crate::theme::resolve_theme_selection("rose-pine-moon", &themes, None, Some(&dir));
    let missing = crate::theme::resolve_theme_selection("rose-pine", &themes, None, Some(&dir));
    std::fs::remove_dir_all(&dir).unwrap();

    let Ok(ThemeSelection::Custom(custom)) = selection else {
        panic!("expected rose-pine-moon to resolve from the themes dir");
    };
    assert_eq!(custom.name, "rose-pine-moon");
    assert_eq!(custom.base_preset, ThemePreset::OceanDark);
    assert_eq!(
        custom.theme.ui.content_bg,
        ratatui::style::Color::Rgb(35, 33, 54)
    );
    assert_eq!(missing.unwrap_err(), "Unknown theme \"rose-pine\"");
}

#[test]
fn themes_dir_does_not_shadow_presets_or_config_themes() {
    let dir = super::unique_temp_dir("leaf-themes-dir-shadow");
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["forest", "midnight"] {
        std::fs::write(dir.join(format!("{name}.toml")), "base = \"arctic\"\n").unwrap();
    }

    let mut themes = std::collections::BTreeMap::new();
    themes.insert(
        "midnight".to_string(),
        toml::from_str::<crate::theme::CustomThemeConfig>("base = \"ocean\"").unwrap(),
    );
    let preset = crate::theme::resolve_theme_selection("forest", &themes, None, Some(&dir));
    let custom = crate::theme::resolve_theme_selection("midnight", &themes, None, Some(&dir));
    std::fs::remove_dir_all(&dir).unwrap();

    assert!(matches!(
        preset,
        Ok(ThemeSelection::Preset(ThemePreset::Forest))
    ));
    let Ok(ThemeSelection::Custom(custom)) = custom else {
        panic!("expected midnight to resolve from [themes.midnight]");
    };
    assert_eq!(custom.base_preset, ThemePreset::OceanDark);
}

#[test]
fn tab_title_n_to_max_filename_len_returns_none_for_disabled() {
    assert_eq!(tab_title_n_to_max_filename_len(-1), None);
    assert_eq!(tab_title_n_to_max_filename_len(-42), None);
    assert_eq!(tab_title_n_to_max_filename_len(0), None);
    assert_eq!(tab_title_n_to_max_filename_len(19), None);
}

#[test]
fn tab_title_n_to_max_filename_len_subtracts_prefix_when_valid() {
    assert_eq!(tab_title_n_to_max_filename_len(20), Some(14));
    assert_eq!(tab_title_n_to_max_filename_len(21), Some(15));
    assert_eq!(tab_title_n_to_max_filename_len(50), Some(44));
}

#[test]
fn parse_tab_title_length_valid_int() {
    let toml = r#"tab-title-length = -1"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.tab_title_length, Some(-1));

    let toml = r#"tab-title-length = 30"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.tab_title_length, Some(30));
}

#[test]
fn parse_tab_title_length_string_falls_back_silently() {
    let toml = r#"
theme = "forest"
tab-title-length = "abc"
"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.tab_title_length, None);
    assert_eq!(config.theme.as_deref(), Some("forest"));
}

#[test]
fn parse_tab_title_length_bool_falls_back_silently() {
    let toml = r#"
theme = "arctic"
tab-title-length = true
"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.tab_title_length, None);
    assert_eq!(config.theme.as_deref(), Some("arctic"));
}

#[test]
fn config_path_returns_some() {
    let path = config_path();
    assert!(path.is_some());
    let path = path.unwrap();
    assert!(path.ends_with("config.toml"));
    assert!(path.to_string_lossy().contains("leaf"));
}

#[test]
fn resolve_tab_title_length_n_none_returns_default_minus_one() {
    let _guard = super::write_theme_test_state();
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(test_resolve_tab_title_length_n(None), Some(-1));
}

#[test]
fn resolve_tab_title_length_n_config_minus_one() {
    let _guard = super::write_theme_test_state();
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(test_resolve_tab_title_length_n(Some(-1)), Some(-1));
}

#[test]
fn resolve_tab_title_length_n_config_valid_value() {
    let _guard = super::write_theme_test_state();
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(test_resolve_tab_title_length_n(Some(30)), Some(30));
}

#[test]
fn resolve_tab_title_length_n_config_invalid_low_returns_none() {
    let _guard = super::write_theme_test_state();
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(test_resolve_tab_title_length_n(Some(5)), None);
}

#[test]
fn resolve_tab_title_length_n_env_override_wins() {
    let _guard = super::write_theme_test_state();
    std::env::set_var("LEAF_TAB_TITLE_LENGTH", "25");
    let result = test_resolve_tab_title_length_n(Some(50));
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(result, Some(25));
}

#[test]
fn resolve_tab_title_length_n_env_invalid_falls_back_to_config() {
    let _guard = super::write_theme_test_state();
    std::env::set_var("LEAF_TAB_TITLE_LENGTH", "not-a-number");
    let result = test_resolve_tab_title_length_n(Some(30));
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(result, Some(30));
}

#[test]
fn parse_main_line_numbers_true() {
    let toml = r#"main-line-numbers = true"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.main_line_numbers, Some(true));
}

#[test]
fn parse_main_line_numbers_false() {
    let toml = r#"main-line-numbers = false"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.main_line_numbers, Some(false));
}

#[test]
fn parse_main_line_numbers_missing_defaults_to_none() {
    let toml = r#"theme = "ocean""#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.main_line_numbers, None);
}

#[test]
fn resolve_main_line_numbers_default_false() {
    std::env::remove_var("LEAF_MAIN_LINE_NUMBERS");
    assert!(!test_resolve_main_line_numbers(None));
}

#[test]
fn resolve_main_line_numbers_config_value() {
    std::env::remove_var("LEAF_MAIN_LINE_NUMBERS");
    assert!(test_resolve_main_line_numbers(Some(true)));
    assert!(!test_resolve_main_line_numbers(Some(false)));
}

#[test]
fn resolve_main_line_numbers_env_var_override() {
    std::env::set_var("LEAF_MAIN_LINE_NUMBERS", "1");
    assert!(test_resolve_main_line_numbers(Some(false)));

    std::env::set_var("LEAF_MAIN_LINE_NUMBERS", "0");
    assert!(!test_resolve_main_line_numbers(Some(true)));

    std::env::set_var("LEAF_MAIN_LINE_NUMBERS", "invalid");
    assert!(test_resolve_main_line_numbers(Some(true)));
    std::env::remove_var("LEAF_MAIN_LINE_NUMBERS");
}

#[test]
fn parse_hyper_link_prefix() {
    let config: LeafConfig = toml::from_str(r#"hyper-link-prefix = """#).unwrap();
    assert_eq!(config.hyper_link_prefix.as_deref(), Some(""));
    let config: LeafConfig = toml::from_str(r#"hyper-link-prefix = "→""#).unwrap();
    assert_eq!(config.hyper_link_prefix.as_deref(), Some("→"));
}

#[test]
fn parse_scroll_with_enter_true() {
    let toml = r#"scroll-with-enter = true"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.scroll_with_enter, Some(true));
}

#[test]
fn parse_scroll_with_enter_false() {
    let toml = r#"scroll-with-enter = false"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.scroll_with_enter, Some(false));
}

#[test]
fn parse_scroll_with_enter_missing_defaults_to_none() {
    let toml = r#"theme = "ocean""#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.scroll_with_enter, None);
}

#[test]
fn parse_page_scroll_size_formats() {
    assert_eq!(
        parse_page_scroll_size("20rows"),
        Some(PageScrollSize::Rows(20))
    );
    assert_eq!(
        parse_page_scroll_size("100%"),
        Some(PageScrollSize::Percent(100))
    );
    assert_eq!(
        parse_page_scroll_size("150%"),
        Some(PageScrollSize::Percent(150))
    );
    assert_eq!(
        parse_page_scroll_size("-3rows"),
        Some(PageScrollSize::KeepRows(3))
    );
}

#[test]
fn parse_page_scroll_size_rejects_invalid_values() {
    for value in [
        "",
        "0rows",
        "-0rows",
        "0%",
        "rows",
        "3",
        "3line",
        "+3rows",
        " 20rows",
        "999999999999999999999999999999999999999999999999999999rows",
    ] {
        assert_eq!(parse_page_scroll_size(value), None, "{value:?}");
    }
}

#[test]
fn page_scroll_size_config_defaults_and_handles_invalid_types() {
    let missing: LeafConfig = toml::from_str("").unwrap();
    assert_eq!(
        missing.resolved_page_scroll_size(),
        PageScrollSize::Rows(20)
    );
    assert_eq!(missing.page_scroll_size, PageScrollSizeSetting::Unset);

    let valid: LeafConfig = toml::from_str(r#"page-scroll-size = "-3rows""#).unwrap();
    assert_eq!(
        valid.page_scroll_size,
        PageScrollSizeSetting::Valid(PageScrollSize::KeepRows(3))
    );

    let invalid: LeafConfig = toml::from_str("page-scroll-size = 20").unwrap();
    assert_eq!(invalid.page_scroll_size, PageScrollSizeSetting::Invalid);
    assert_eq!(
        invalid.resolved_page_scroll_size(),
        PageScrollSize::Rows(20)
    );
}

#[test]
fn validate_hyper_link_prefix_graphemes() {
    use crate::config::is_valid_hyper_link_prefix;
    assert!(is_valid_hyper_link_prefix(""));
    assert!(is_valid_hyper_link_prefix("#"));
    assert!(is_valid_hyper_link_prefix("→"));
    assert!(is_valid_hyper_link_prefix("e\u{0301}"));
    assert!(!is_valid_hyper_link_prefix("ab"));
    assert!(!is_valid_hyper_link_prefix(" "));
    assert!(!is_valid_hyper_link_prefix("→→"));
}

#[test]
fn parse_keep_soft_breaks_true() {
    let toml = r#"keep-soft-breaks = true"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.keep_soft_breaks, Some(true));
}

#[test]
fn parse_keep_soft_breaks_missing_defaults_to_none() {
    let toml = r#"theme = "ocean""#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.keep_soft_breaks, None);
}
