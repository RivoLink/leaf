use ratatui::style::Color;
use serde::{
    de::{Error as DeError, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

use super::{resolution::parse_theme_color, MarkdownTheme, UiTheme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ThemeColor(pub(crate) Color);

impl<'de> Deserialize<'de> for ThemeColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorVisitor;

        impl<'de> Visitor<'de> for ColorVisitor {
            type Value = ThemeColor;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a color name, hex color string, rgb(...) string, or [r, g, b]")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: DeError,
            {
                parse_theme_color(value)
                    .map(ThemeColor)
                    .ok_or_else(|| E::custom(format!("invalid theme color: {value}")))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: DeError,
            {
                self.visit_str(&value)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let r = read_rgb_component(&mut seq, "red")?;
                let g = read_rgb_component(&mut seq, "green")?;
                let b = read_rgb_component(&mut seq, "blue")?;
                if seq.next_element::<u16>()?.is_some() {
                    return Err(A::Error::custom(
                        "theme RGB array must contain exactly 3 values",
                    ));
                }
                Ok(ThemeColor(Color::Rgb(r, g, b)))
            }
        }

        deserializer.deserialize_any(ColorVisitor)
    }
}

pub(super) fn read_rgb_component<'de, A>(seq: &mut A, name: &str) -> Result<u8, A::Error>
where
    A: SeqAccess<'de>,
{
    let value = seq
        .next_element::<u16>()?
        .ok_or_else(|| A::Error::custom(format!("missing {name} theme color component")))?;
    u8::try_from(value)
        .map_err(|_| A::Error::custom(format!("{name} theme color component out of range")))
}

macro_rules! theme_overrides {
    ($name:ident for $theme:ty { $($field:ident),+ $(,)? }) => {
        #[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
        #[serde(default)]
        pub(crate) struct $name {
            $(pub(crate) $field: Option<ThemeColor>,)+
        }

        impl $name {
            pub(super) fn apply_to(&self, theme: &mut $theme) {
                $(
                    if let Some(color) = self.$field {
                        theme.$field = color.0;
                    }
                )+
            }
        }
    };
}

theme_overrides!(UiThemeOverrides for UiTheme {
    toc_bg,
    toc_border,
    content_bg,
    scrollbar_hover,
    status_bg,
    status_separator,
    status_brand_fg,
    status_brand_bg,
    status_filename_fg,
    status_filename_bg,
    status_watch_fg,
    status_watch_bg,
    status_reloaded_fg,
    status_reloaded_bg,
    status_search_fg,
    status_search_bg,
    status_success_fg,
    status_success_bg,
    status_warning_fg,
    status_error_fg,
    status_error_bg,
    status_shortcut_fg,
    status_percent_fg,
    toc_hover_fg,
    toc_header_fg,
    toc_active_bg,
    toc_inactive_bg,
    toc_accent,
    toc_index_inactive,
    toc_primary_active,
    toc_primary_inactive,
    toc_secondary_inactive,
    toc_secondary_text_active,
    toc_secondary_text_inactive,
});

theme_overrides!(MarkdownThemeOverrides for MarkdownTheme {
    search_highlight_bg,
    search_match_bg,
    code_gutter,
    blockquote_marker,
    list_level_1,
    list_level_2,
    list_level_3,
    ordered_list,
    table_border,
    table_separator,
    table_header,
    table_cell,
    heading_1,
    heading_2,
    heading_3,
    heading_4,
    heading_other,
    heading_underline,
    code_frame,
    code_label,
    inline_code_fg,
    inline_code_bg,
    rule,
    link_icon,
    link_text,
    link_hover,
    blockquote_text,
    text,
    strong_text,
    latex_inline_fg,
    latex_inline_bg,
    latex_block_fg,
    mermaid_keyword,
    mermaid_arrow,
    mermaid_label,
    mermaid_block_fg,
    mark_fg,
    mark_bg,
    task_checked,
    task_unchecked,
    alert_note,
    alert_tip,
    alert_important,
    alert_warning,
    alert_caution,
    footnote_ref,
    footnote_text,
});

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct CustomThemeConfig {
    pub(crate) base: Option<String>,
    pub(crate) syntax: Option<String>,
    pub(crate) ui: UiThemeOverrides,
    pub(crate) markdown: MarkdownThemeOverrides,
}
