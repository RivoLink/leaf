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
        crate::theme::resolve_theme_selection("gruvbox.toml", &themes, Some(&dir)).unwrap();
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
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(test_resolve_tab_title_length_n(None), Some(-1));
}

#[test]
fn resolve_tab_title_length_n_config_minus_one() {
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(test_resolve_tab_title_length_n(Some(-1)), Some(-1));
}

#[test]
fn resolve_tab_title_length_n_config_valid_value() {
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(test_resolve_tab_title_length_n(Some(30)), Some(30));
}

#[test]
fn resolve_tab_title_length_n_config_invalid_low_returns_none() {
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(test_resolve_tab_title_length_n(Some(5)), None);
}

#[test]
fn resolve_tab_title_length_n_env_override_wins() {
    std::env::set_var("LEAF_TAB_TITLE_LENGTH", "25");
    let result = test_resolve_tab_title_length_n(Some(50));
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(result, Some(25));
}

#[test]
fn resolve_tab_title_length_n_env_invalid_falls_back_to_config() {
    std::env::set_var("LEAF_TAB_TITLE_LENGTH", "not-a-number");
    let result = test_resolve_tab_title_length_n(Some(30));
    std::env::remove_var("LEAF_TAB_TITLE_LENGTH");
    assert_eq!(result, Some(30));
}
