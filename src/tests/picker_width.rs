use crate::app::App;
use crate::picker_width::{
    parse_picker_width_spec, picker_width_raw, PickerWidthSpec, DEFAULT_PICKER_WIDTH,
    PICKER_WIDTH_FLOOR_CELLS,
};
use crate::LeafConfig;
use ratatui::text::Line;

fn effective_cells(spec: PickerWidthSpec, area_width: u16) -> u16 {
    picker_width_raw(spec, area_width).max(PICKER_WIDTH_FLOOR_CELLS)
}

#[test]
fn parse_picker_width_percent() {
    assert_eq!(
        parse_picker_width_spec("1%"),
        Some(PickerWidthSpec::Percent(1))
    );
    assert_eq!(
        parse_picker_width_spec("50%"),
        Some(PickerWidthSpec::Percent(50))
    );
    assert_eq!(
        parse_picker_width_spec("78%"),
        Some(PickerWidthSpec::Percent(78))
    );
    assert_eq!(
        parse_picker_width_spec("100%"),
        Some(PickerWidthSpec::Percent(100))
    );
}

#[test]
fn parse_picker_width_cells() {
    assert_eq!(
        parse_picker_width_spec("1cell"),
        Some(PickerWidthSpec::Cells(1))
    );
    assert_eq!(
        parse_picker_width_spec("78cells"),
        Some(PickerWidthSpec::Cells(78))
    );
    assert_eq!(
        parse_picker_width_spec("200cells"),
        Some(PickerWidthSpec::Cells(200))
    );
}

#[test]
fn parse_picker_width_case_insensitive() {
    assert_eq!(
        parse_picker_width_spec("78Cells"),
        Some(PickerWidthSpec::Cells(78))
    );
    assert_eq!(
        parse_picker_width_spec("78CELLS"),
        Some(PickerWidthSpec::Cells(78))
    );
    assert_eq!(
        parse_picker_width_spec("50%"),
        Some(PickerWidthSpec::Percent(50))
    );
}

#[test]
fn parse_picker_width_trims_whitespace() {
    assert_eq!(
        parse_picker_width_spec(" 78cells "),
        Some(PickerWidthSpec::Cells(78))
    );
    assert_eq!(
        parse_picker_width_spec("\t50%\n"),
        Some(PickerWidthSpec::Percent(50))
    );
}

#[test]
fn parse_picker_width_rejects_internal_spaces() {
    assert_eq!(parse_picker_width_spec("78 cells"), None);
    assert_eq!(parse_picker_width_spec("50 %"), None);
}

#[test]
fn parse_picker_width_invalid_returns_none() {
    assert_eq!(parse_picker_width_spec("abc"), None);
    assert_eq!(parse_picker_width_spec(""), None);
    assert_eq!(parse_picker_width_spec("78"), None);
    assert_eq!(parse_picker_width_spec("78px"), None);
    assert_eq!(parse_picker_width_spec("0%"), None);
    assert_eq!(parse_picker_width_spec("101%"), None);
    assert_eq!(parse_picker_width_spec("0cells"), None);
}

#[test]
fn parse_picker_width_deserialize_int_returns_none() {
    let toml = r#"file-picker-width = 78"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.file_picker_width, None);
}

#[test]
fn parse_picker_width_deserialize_bool_returns_none() {
    let toml = r#"file-picker-width = true"#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.file_picker_width, None);
}

#[test]
fn parse_picker_width_deserialize_string_ok() {
    let toml = r#"file-picker-width = "50%""#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.file_picker_width, Some(PickerWidthSpec::Percent(50)));

    let toml = r#"file-picker-width = "78cells""#;
    let config: LeafConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.file_picker_width, Some(PickerWidthSpec::Cells(78)));
}

#[test]
fn resolve_file_picker_width_defaults_when_none() {
    // Note: env var is not set in test env by default.
    let _guard = crate::tests::THEME_TEST_MUTEX.lock().unwrap();
    std::env::remove_var("LEAF_FILE_PICKER_WIDTH");
    let result = crate::resolve_file_picker_width(None);
    assert_eq!(result, DEFAULT_PICKER_WIDTH);
}

#[test]
fn resolve_file_picker_width_returns_config_when_valid() {
    let _guard = crate::tests::THEME_TEST_MUTEX.lock().unwrap();
    std::env::remove_var("LEAF_FILE_PICKER_WIDTH");
    let result = crate::resolve_file_picker_width(Some(PickerWidthSpec::Percent(60)));
    assert_eq!(result, PickerWidthSpec::Percent(60));
}

#[test]
fn resolve_file_picker_width_env_overrides_config() {
    let _guard = crate::tests::THEME_TEST_MUTEX.lock().unwrap();
    std::env::set_var("LEAF_FILE_PICKER_WIDTH", "42cells");
    let result = crate::resolve_file_picker_width(Some(PickerWidthSpec::Percent(60)));
    std::env::remove_var("LEAF_FILE_PICKER_WIDTH");
    assert_eq!(result, PickerWidthSpec::Cells(42));
}

#[test]
fn resolve_file_picker_width_falls_back_when_env_invalid() {
    let _guard = crate::tests::THEME_TEST_MUTEX.lock().unwrap();
    std::env::set_var("LEAF_FILE_PICKER_WIDTH", "garbage");
    let result = crate::resolve_file_picker_width(Some(PickerWidthSpec::Percent(60)));
    std::env::remove_var("LEAF_FILE_PICKER_WIDTH");
    assert_eq!(result, PickerWidthSpec::Percent(60));
}

#[test]
fn picker_width_cells_percent_ceils_correctly() {
    assert_eq!(effective_cells(PickerWidthSpec::Percent(80), 101), 81);
    assert_eq!(effective_cells(PickerWidthSpec::Percent(50), 201), 101);
}

#[test]
fn picker_width_cells_percent_applies_floor() {
    assert_eq!(effective_cells(PickerWidthSpec::Percent(50), 100), 78);
    assert_eq!(effective_cells(PickerWidthSpec::Percent(1), 100), 78);
}

#[test]
fn picker_width_cells_percent_no_floor_when_above() {
    assert_eq!(effective_cells(PickerWidthSpec::Percent(50), 200), 100);
    assert_eq!(effective_cells(PickerWidthSpec::Percent(100), 200), 200);
}

#[test]
fn picker_width_cells_cells_applies_floor() {
    assert_eq!(effective_cells(PickerWidthSpec::Cells(30), 200), 78);
    assert_eq!(effective_cells(PickerWidthSpec::Cells(1), 200), 78);
}

#[test]
fn picker_width_cells_cells_pass_through_above_floor() {
    assert_eq!(effective_cells(PickerWidthSpec::Cells(120), 200), 120);
    assert_eq!(effective_cells(PickerWidthSpec::Cells(78), 200), 78);
}

#[test]
fn picker_width_floor_constant_is_78() {
    assert_eq!(PICKER_WIDTH_FLOOR_CELLS, 78);
}

fn make_app_with_spec(spec: PickerWidthSpec) -> App {
    let mut app = App::new(
        vec![Line::from("test")],
        vec![],
        "test".to_string(),
        false,
        false,
        None,
        None,
    );
    app.set_file_picker_width(spec);
    app
}

#[test]
fn floor_state_triggers_warning_on_transition_below() {
    let mut app = make_app_with_spec(PickerWidthSpec::Percent(50));
    assert!(!app.picker_width_floor_active);
    app.refresh_picker_width_floor(100);
    assert!(app.picker_width_floor_active);
    assert!(app.config_flash().is_some());
}

#[test]
fn floor_state_resets_silently_on_transition_above() {
    let mut app = make_app_with_spec(PickerWidthSpec::Percent(50));
    app.refresh_picker_width_floor(100);
    assert!(app.picker_width_floor_active);
    app.refresh_picker_width_floor(200);
    assert!(!app.picker_width_floor_active);
}

#[test]
fn floor_state_re_triggers_after_reset() {
    let mut app = make_app_with_spec(PickerWidthSpec::Percent(50));
    app.refresh_picker_width_floor(100);
    assert!(app.picker_width_floor_active);
    app.refresh_picker_width_floor(200);
    assert!(!app.picker_width_floor_active);
    app.clear_config_flash();
    app.refresh_picker_width_floor(100);
    assert!(app.picker_width_floor_active);
    assert!(app.config_flash().is_some());
}

#[test]
fn floor_state_triggers_for_cells_spec_below_floor() {
    let mut app = make_app_with_spec(PickerWidthSpec::Cells(30));
    app.refresh_picker_width_floor(100);
    assert!(app.picker_width_floor_active);
}

#[test]
fn floor_state_no_trigger_for_cells_spec_above_floor() {
    let mut app = make_app_with_spec(PickerWidthSpec::Cells(120));
    app.refresh_picker_width_floor(200);
    assert!(!app.picker_width_floor_active);
}

#[test]
fn floor_state_no_trigger_when_percent_produces_above_floor() {
    let mut app = make_app_with_spec(PickerWidthSpec::Percent(50));
    app.refresh_picker_width_floor(200);
    assert!(!app.picker_width_floor_active);
}

#[test]
fn floor_state_no_trigger_when_clamp_equalizes() {
    let mut app = make_app_with_spec(PickerWidthSpec::Cells(30));
    // On terminal 30: cap=28, min(30,28)=28, min(78,28)=28 → equal, silence.
    app.refresh_picker_width_floor(30);
    assert!(!app.picker_width_floor_active);
}

#[test]
fn floor_state_cycle_triggers_on_return_to_visible_effect() {
    let mut app = make_app_with_spec(PickerWidthSpec::Cells(30));
    app.refresh_picker_width_floor(100);
    assert!(app.picker_width_floor_active);
    app.refresh_picker_width_floor(30);
    assert!(!app.picker_width_floor_active);
    app.clear_config_flash();
    app.refresh_picker_width_floor(100);
    assert!(app.picker_width_floor_active);
    assert!(app.config_flash().is_some());
}

#[test]
fn floor_state_reset_on_close_file_picker() {
    let mut app = make_app_with_spec(PickerWidthSpec::Cells(30));
    app.refresh_picker_width_floor(100);
    assert!(app.picker_width_floor_active);
    app.close_file_picker();
    assert!(!app.picker_width_floor_active);
}

#[test]
fn floor_state_reset_on_cancel_picker_loading() {
    let mut app = make_app_with_spec(PickerWidthSpec::Cells(30));
    app.refresh_picker_width_floor(100);
    assert!(app.picker_width_floor_active);
    app.cancel_picker_loading();
    assert!(!app.picker_width_floor_active);
}
