use crate::{
    markdown::{parse_markdown_with_width, toc::TocEntry},
    theme::{
        app_theme, current_syntect_theme, current_theme_selection, set_theme_preset,
        set_theme_selection, theme_preset_index, ThemePreset, ThemeSelection, THEME_PRESETS,
    },
};
use ratatui::text::Line;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

use super::App;

#[derive(Clone)]
pub(crate) struct ThemePreviewCacheEntry {
    pub(super) lines: Vec<Line<'static>>,
    pub(super) toc: Vec<TocEntry>,
}

pub(crate) struct ThemePickerState {
    pub(super) open: bool,
    pub(super) index: usize,
    pub(super) original: Option<ThemeSelection>,
    pub(super) original_preview: Option<ThemePreviewCacheEntry>,
    pub(super) preview_cache: Vec<Option<ThemePreviewCacheEntry>>,
}

impl App {
    pub(crate) fn open_theme_picker(&mut self) {
        self.theme_picker.open = true;
        let current = current_theme_selection();
        self.theme_picker.index = theme_preset_index(current.preset_hint());
        self.theme_picker.original = Some(current);
        self.theme_picker.original_preview = Some(ThemePreviewCacheEntry {
            lines: self.lines.clone(),
            toc: self.toc.clone(),
        });
        self.store_current_theme_preview();
    }

    pub(crate) fn close_theme_picker(&mut self) {
        self.theme_picker.open = false;
        self.theme_picker.original = None;
        self.theme_picker.original_preview = None;
    }

    pub(crate) fn is_theme_picker_open(&self) -> bool {
        self.theme_picker.open
    }

    pub(crate) fn theme_picker_index(&self) -> usize {
        self.theme_picker.index
    }

    #[cfg(test)]
    pub(crate) fn theme_picker_original(&self) -> Option<ThemeSelection> {
        self.theme_picker.original.clone()
    }

    pub(crate) fn theme_picker_reference_preset(&self) -> Option<ThemePreset> {
        self.theme_picker
            .original
            .as_ref()
            .and_then(ThemeSelection::as_preset)
            .or_else(|| current_theme_selection().as_preset())
    }

    pub(crate) fn move_theme_picker_up(&mut self) {
        let total = THEME_PRESETS.len();
        if total == 0 {
            return;
        }
        if self.theme_picker.index == 0 {
            self.theme_picker.index = total - 1;
        } else {
            self.theme_picker.index -= 1;
        }
    }

    pub(crate) fn move_theme_picker_down(&mut self) {
        let total = THEME_PRESETS.len();
        if total == 0 {
            return;
        }
        self.theme_picker.index = (self.theme_picker.index + 1) % total;
    }

    pub(crate) fn set_theme_picker_index(&mut self, idx: usize) -> bool {
        if idx < THEME_PRESETS.len() {
            self.theme_picker.index = idx;
            true
        } else {
            false
        }
    }

    pub(crate) fn selected_theme_preset(&self) -> Option<ThemePreset> {
        THEME_PRESETS.get(self.theme_picker.index).copied()
    }

    pub(crate) fn preview_theme_preset(
        &mut self,
        preset: ThemePreset,
        ss: &SyntaxSet,
        themes: &ThemeSet,
    ) {
        if current_theme_selection().as_preset() == Some(preset) {
            return;
        }
        set_theme_preset(preset);
        let cached = self
            .theme_picker
            .preview_cache
            .get(theme_preset_index(preset))
            .and_then(|entry| entry.as_ref())
            .cloned();
        if let Some(entry) = cached {
            self.replace_content(crate::markdown::ParseResult::preview(
                entry.lines,
                entry.toc,
            ));
            return;
        }

        let theme = current_syntect_theme(themes);
        let at = app_theme();
        let base_path = self.filepath.as_deref().and_then(|p| p.parent());
        let parsed = parse_markdown_with_width(
            &self.source,
            ss,
            theme,
            self.render_width,
            &at.markdown,
            self.file_mode,
            self.code_line_numbers,
            base_path,
            &self.image_picker,
        );
        self.store_theme_preview(preset, &parsed.lines, &parsed.toc);
        self.replace_content(parsed);
    }

    pub(crate) fn restore_theme_picker_preview(&mut self, ss: &SyntaxSet, themes: &ThemeSet) {
        if let Some(original) = self.theme_picker.original.take() {
            set_theme_selection(original);
            if let Some(entry) = self.theme_picker.original_preview.take() {
                self.replace_content(crate::markdown::ParseResult::preview(
                    entry.lines,
                    entry.toc,
                ));
            } else {
                let theme = current_syntect_theme(themes);
                let at = app_theme();
                let base_path = self.filepath.as_deref().and_then(|p| p.parent());
                let parsed = parse_markdown_with_width(
                    &self.source,
                    ss,
                    theme,
                    self.render_width,
                    &at.markdown,
                    self.file_mode,
                    self.code_line_numbers,
                    base_path,
                    &self.image_picker,
                );
                self.replace_content(parsed);
            }
        }
        self.close_theme_picker();
    }

    pub(crate) fn store_theme_preview(
        &mut self,
        preset: ThemePreset,
        lines: &[Line<'static>],
        toc: &[TocEntry],
    ) {
        let idx = theme_preset_index(preset);
        if let Some(slot) = self.theme_picker.preview_cache.get_mut(idx) {
            *slot = Some(ThemePreviewCacheEntry {
                lines: lines.to_vec(),
                toc: toc.to_vec(),
            });
        }
    }

    pub(crate) fn store_current_theme_preview(&mut self) {
        let Some(preset) = current_theme_selection().as_preset() else {
            return;
        };
        let idx = theme_preset_index(preset);
        if let Some(slot) = self.theme_picker.preview_cache.get_mut(idx) {
            *slot = Some(ThemePreviewCacheEntry {
                lines: self.lines.clone(),
                toc: self.toc.clone(),
            });
        }
    }

    pub(crate) fn store_current_theme_preview_from(
        &mut self,
        lines: &[Line<'static>],
        toc: &[TocEntry],
    ) {
        let Some(preset) = current_theme_selection().as_preset() else {
            return;
        };
        self.store_theme_preview(preset, lines, toc);
    }

    pub(crate) fn invalidate_theme_preview_cache(&mut self) {
        self.theme_picker.preview_cache.fill(None);
    }

    #[cfg(test)]
    pub(crate) fn has_cached_theme_preview(&self, preset: ThemePreset) -> bool {
        self.theme_picker
            .preview_cache
            .get(theme_preset_index(preset))
            .and_then(|entry| entry.as_ref())
            .is_some()
    }
}
