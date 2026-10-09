use ratatui::style::Color;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    path::{Path, PathBuf},
    str::FromStr,
};
use syntect::highlighting::ThemeSet;

use super::{
    parse_theme_preset, theme_by_preset, CustomTheme, CustomThemeConfig, ThemeSelection,
    DEFAULT_PRESET,
};

pub(crate) fn parse_theme_color(value: &str) -> Option<Color> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(color) = parse_hex_color(value) {
        return Some(color);
    }
    if let Some(color) = parse_rgb_color(value) {
        return Some(color);
    }

    Color::from_str(value).ok()
}

pub(super) fn parse_hex_color(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    match hex.len() {
        3 if hex.chars().all(|c| c.is_ascii_hexdigit()) => {
            let mut expanded = String::with_capacity(6);
            for ch in hex.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            parse_hex_color(&expanded)
        }
        6 if hex.chars().all(|c| c.is_ascii_hexdigit()) => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

pub(super) fn parse_rgb_color(value: &str) -> Option<Color> {
    let value = value.trim();
    let inner = value.strip_prefix("rgb(")?.strip_suffix(')')?;
    let mut components = inner.split(',').map(str::trim);
    let r = components.next()?.parse::<u8>().ok()?;
    let g = components.next()?.parse::<u8>().ok()?;
    let b = components.next()?.parse::<u8>().ok()?;
    if components.next().is_some() {
        return None;
    }
    Some(Color::Rgb(r, g, b))
}

pub(crate) fn resolve_theme_selection(
    name: &str,
    custom_themes: &BTreeMap<String, CustomThemeConfig>,
    theme_file_base_dir: Option<&Path>,
) -> Result<ThemeSelection, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Theme name cannot be empty".to_string());
    }

    if let Some(preset) = parse_theme_preset(name) {
        return Ok(ThemeSelection::Preset(preset));
    }

    if let Some(custom_config) = custom_themes.get(name) {
        return custom_theme_selection(name, custom_config);
    }

    if looks_like_theme_file(name) {
        return resolve_theme_file_selection(name, theme_file_base_dir);
    }

    Err(format!("Unknown theme \"{name}\""))
}

pub(super) fn custom_theme_selection(
    name: &str,
    custom_config: &CustomThemeConfig,
) -> Result<ThemeSelection, String> {
    let base_preset = match custom_config.base.as_deref() {
        Some(base) => parse_theme_preset(base)
            .ok_or_else(|| format!("Unknown base theme \"{base}\" for custom theme \"{name}\""))?,
        None => DEFAULT_PRESET,
    };

    let mut theme = theme_by_preset(base_preset).clone();
    if let Some(syntax) = custom_config.syntax.as_deref() {
        let syntax = syntax.trim();
        if syntax.is_empty() {
            return Err(format!("Empty syntax theme for custom theme \"{name}\""));
        }
        theme.syntax_theme_name = Cow::Owned(syntax.to_string());
    }
    custom_config.ui.apply_to(&mut theme.ui);
    custom_config.markdown.apply_to(&mut theme.markdown);
    custom_config.diff.apply_to(&mut theme.diff);

    Ok(ThemeSelection::Custom(Box::new(CustomTheme {
        name: name.to_string(),
        base_preset,
        theme,
    })))
}

pub(super) fn looks_like_theme_file(name: &str) -> bool {
    name.ends_with(".toml") || name.contains('/') || name.contains('\\')
}

pub(super) fn resolve_theme_file_selection(
    name: &str,
    theme_file_base_dir: Option<&Path>,
) -> Result<ThemeSelection, String> {
    let path = resolve_theme_file_path(name, theme_file_base_dir);
    let content = std::fs::read_to_string(&path)
        .map_err(|err| format!("Cannot read theme file \"{}\": {err}", path.display()))?;
    let custom_config = toml::from_str::<CustomThemeConfig>(&content)
        .map_err(|err| format!("Could not parse theme file \"{}\": {err}", path.display()))?;
    let theme_name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or(name);
    custom_theme_selection(theme_name, &custom_config)
}

pub(super) fn resolve_theme_file_path(name: &str, theme_file_base_dir: Option<&Path>) -> PathBuf {
    let path = Path::new(name);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    theme_file_base_dir
        .map(|base_dir| base_dir.join(path))
        .unwrap_or_else(|| path.to_path_buf())
}

pub(crate) fn validate_theme_syntax(
    selection: &ThemeSelection,
    themes: &ThemeSet,
) -> Option<String> {
    let syntax_theme_name = selection.syntax_theme_name();
    (!themes.themes.contains_key(syntax_theme_name)).then(|| {
        format!("Unknown syntax theme \"{syntax_theme_name}\", using default syntax colors")
    })
}
