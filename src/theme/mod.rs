mod presets;
mod resolution;
mod serde;

use ratatui::style::Color;
use std::{borrow::Cow, sync::RwLock};
use syntect::{highlighting::Theme, highlighting::ThemeSet};

use presets::{ARCTIC_THEME, FOREST_THEME, OCEAN_DARK_THEME, SOLARIZED_DARK_THEME};

pub(crate) use self::serde::CustomThemeConfig;
#[cfg(test)]
pub(crate) use resolution::parse_theme_color;
pub(crate) use resolution::{resolve_theme_selection, validate_theme_syntax};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ThemePreset {
    Arctic = 0,
    Forest = 1,
    OceanDark = 2,
    SolarizedDark = 3,
}

impl Default for ThemePreset {
    fn default() -> Self {
        DEFAULT_PRESET
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AppTheme {
    pub(crate) syntax_theme_name: Cow<'static, str>,
    pub(crate) ui: UiTheme,
    pub(crate) markdown: MarkdownTheme,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct UiTheme {
    pub(crate) toc_bg: Color,
    pub(crate) toc_border: Color,
    pub(crate) content_bg: Color,
    pub(crate) scrollbar_hover: Color,
    pub(crate) status_bg: Color,
    pub(crate) status_separator: Color,
    pub(crate) status_brand_fg: Color,
    pub(crate) status_brand_bg: Color,
    pub(crate) status_filename_fg: Color,
    pub(crate) status_filename_bg: Color,
    pub(crate) status_watch_fg: Color,
    pub(crate) status_watch_bg: Color,
    pub(crate) status_reloaded_fg: Color,
    pub(crate) status_reloaded_bg: Color,
    pub(crate) status_search_fg: Color,
    pub(crate) status_search_bg: Color,
    pub(crate) status_success_fg: Color,
    pub(crate) status_success_bg: Color,
    pub(crate) status_warning_fg: Color,
    pub(crate) status_error_fg: Color,
    pub(crate) status_error_bg: Color,
    pub(crate) status_shortcut_fg: Color,
    pub(crate) status_percent_fg: Color,
    pub(crate) toc_hover_fg: Color,
    pub(crate) toc_header_fg: Color,
    pub(crate) toc_active_bg: Color,
    pub(crate) toc_inactive_bg: Color,
    pub(crate) toc_accent: Color,
    pub(crate) toc_index_inactive: Color,
    pub(crate) toc_primary_active: Color,
    pub(crate) toc_primary_inactive: Color,
    pub(crate) toc_secondary_inactive: Color,
    pub(crate) toc_secondary_text_active: Color,
    pub(crate) toc_secondary_text_inactive: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MarkdownTheme {
    pub(crate) search_highlight_bg: Color,
    pub(crate) search_match_bg: Color,
    pub(crate) code_gutter: Color,
    pub(crate) blockquote_marker: Color,
    pub(crate) list_level_1: Color,
    pub(crate) list_level_2: Color,
    pub(crate) list_level_3: Color,
    pub(crate) ordered_list: Color,
    pub(crate) table_border: Color,
    pub(crate) table_separator: Color,
    pub(crate) table_header: Color,
    pub(crate) table_cell: Color,
    pub(crate) heading_1: Color,
    pub(crate) heading_2: Color,
    pub(crate) heading_3: Color,
    pub(crate) heading_4: Color,
    pub(crate) heading_other: Color,
    pub(crate) heading_underline: Color,
    pub(crate) code_frame: Color,
    pub(crate) code_label: Color,
    pub(crate) inline_code_fg: Color,
    pub(crate) inline_code_bg: Color,
    pub(crate) rule: Color,
    pub(crate) link_icon: Color,
    pub(crate) link_text: Color,
    pub(crate) link_hover: Color,
    pub(crate) blockquote_text: Color,
    pub(crate) text: Color,
    pub(crate) strong_text: Color,
    pub(crate) latex_inline_fg: Color,
    pub(crate) latex_inline_bg: Color,
    pub(crate) latex_block_fg: Color,
    pub(crate) mermaid_keyword: Color,
    pub(crate) mermaid_arrow: Color,
    pub(crate) mermaid_label: Color,
    pub(crate) mermaid_block_fg: Color,
    pub(crate) mark_fg: Color,
    pub(crate) mark_bg: Color,
    pub(crate) task_checked: Color,
    pub(crate) task_unchecked: Color,
    pub(crate) alert_note: Color,
    pub(crate) alert_tip: Color,
    pub(crate) alert_important: Color,
    pub(crate) alert_warning: Color,
    pub(crate) alert_caution: Color,
    pub(crate) footnote_ref: Color,
    pub(crate) footnote_text: Color,
}

pub(crate) const DEFAULT_PRESET: ThemePreset = ThemePreset::OceanDark;
pub(crate) const THEME_PRESETS: [ThemePreset; 4] = [
    ThemePreset::Arctic,
    ThemePreset::Forest,
    ThemePreset::OceanDark,
    ThemePreset::SolarizedDark,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CustomTheme {
    pub(crate) name: String,
    pub(crate) base_preset: ThemePreset,
    pub(crate) theme: AppTheme,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ThemeSelection {
    Preset(ThemePreset),
    Custom(Box<CustomTheme>),
}

impl Default for ThemeSelection {
    fn default() -> Self {
        Self::Preset(DEFAULT_PRESET)
    }
}

impl ThemeSelection {
    pub(crate) fn as_preset(&self) -> Option<ThemePreset> {
        match self {
            Self::Preset(preset) => Some(*preset),
            Self::Custom(_) => None,
        }
    }

    pub(crate) fn preset_hint(&self) -> ThemePreset {
        match self {
            Self::Preset(preset) => *preset,
            Self::Custom(custom) => custom.base_preset,
        }
    }

    pub(crate) fn app_theme(&self) -> AppTheme {
        match self {
            Self::Preset(preset) => theme_by_preset(*preset).clone(),
            Self::Custom(custom) => custom.theme.clone(),
        }
    }

    pub(crate) fn syntax_theme_name(&self) -> &str {
        match self {
            Self::Preset(preset) => theme_by_preset(*preset).syntax_theme_name.as_ref(),
            Self::Custom(custom) => custom.theme.syntax_theme_name.as_ref(),
        }
    }
}

static CURRENT_THEME: RwLock<ThemeSelection> = RwLock::new(ThemeSelection::Preset(DEFAULT_PRESET));

pub(crate) fn parse_theme_preset(name: &str) -> Option<ThemePreset> {
    match name {
        "arctic" => Some(ThemePreset::Arctic),
        "ocean" | "ocean-dark" | "dark" => Some(ThemePreset::OceanDark),
        "forest" => Some(ThemePreset::Forest),
        "solarized" | "solarized-dark" => Some(ThemePreset::SolarizedDark),
        _ => None,
    }
}

pub(crate) fn theme_preset_label(preset: ThemePreset) -> &'static str {
    match preset {
        ThemePreset::Arctic => "Arctic",
        ThemePreset::OceanDark => "Ocean Dark",
        ThemePreset::Forest => "Forest",
        ThemePreset::SolarizedDark => "Solarized Dark",
    }
}

pub(crate) fn theme_preset_index(preset: ThemePreset) -> usize {
    THEME_PRESETS
        .iter()
        .position(|candidate| *candidate == preset)
        .unwrap_or(0)
}

pub(crate) fn theme_by_preset(preset: ThemePreset) -> &'static AppTheme {
    match preset {
        ThemePreset::Arctic => &ARCTIC_THEME,
        ThemePreset::OceanDark => &OCEAN_DARK_THEME,
        ThemePreset::Forest => &FOREST_THEME,
        ThemePreset::SolarizedDark => &SOLARIZED_DARK_THEME,
    }
}

pub(crate) fn set_theme_selection(selection: ThemeSelection) {
    *CURRENT_THEME.write().expect("theme state lock poisoned") = selection;
}

pub(crate) fn current_theme_selection() -> ThemeSelection {
    CURRENT_THEME
        .read()
        .expect("theme state lock poisoned")
        .clone()
}

pub(crate) fn set_theme_preset(preset: ThemePreset) {
    set_theme_selection(ThemeSelection::Preset(preset));
}

#[cfg(test)]
pub(crate) fn current_theme_preset() -> ThemePreset {
    current_theme_selection().preset_hint()
}

pub(crate) fn app_theme() -> AppTheme {
    current_theme_selection().app_theme()
}

pub(crate) fn current_syntect_theme(themes: &ThemeSet) -> &Theme {
    let selection = current_theme_selection();
    let name = selection.syntax_theme_name();
    themes
        .themes
        .get(name)
        .or_else(|| {
            themes
                .themes
                .get(theme_by_preset(DEFAULT_PRESET).syntax_theme_name.as_ref())
        })
        .or_else(|| themes.themes.values().next())
        .expect("syntect theme set is empty")
}
