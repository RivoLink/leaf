use super::App;
use crate::markdown::{
    hash_file_contents, hash_str, parse_markdown_with_width, read_file_state, ParseResult,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

use crate::theme::{app_theme, current_syntect_theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileState {
    pub(crate) modified: SystemTime,
    pub(crate) len: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileChange {
    Metadata(FileState),
    Content(FileState),
}

impl App {
    pub(crate) fn replace_content(&mut self, parsed: ParseResult) {
        use crate::markdown::build_searchable_lines;
        use crate::render::toc_header_line;

        let ParseResult {
            lines,
            toc,
            link_spans,
            link_urls,
            line_number_map,
            source_line_map,
            paragraph_map,
            code_blocks,
        } = parsed;

        self.plain_lines = build_searchable_lines(&lines)
            .into_iter()
            .map(|line| line.to_lowercase())
            .collect();
        self.lines = lines;
        self.toc = toc;
        self.highlighted_line_cache = None;
        self.toc_header_line = toc_header_line();
        self.set_links(link_urls, link_spans);
        self.hovered_link = None;
        self.copy_highlight = None;
        self.set_code_blocks(code_blocks);
        self.code_select = None;
        self.set_line_maps(line_number_map, source_line_map, paragraph_map);
        self.refresh_static_caches();
    }

    pub(super) fn content_snapshot(&self) -> ParseResult {
        ParseResult {
            lines: self.lines.clone(),
            toc: self.toc.clone(),
            link_spans: self
                .link_spans_by_line
                .values()
                .flatten()
                .cloned()
                .collect(),
            link_urls: self.link_urls.clone(),
            line_number_map: self.line_number_map.clone(),
            source_line_map: self.source_line_map.clone(),
            paragraph_map: self.paragraph_map.clone(),
            code_blocks: self.code_blocks.clone(),
        }
    }

    pub(crate) fn reload(&mut self, ss: &SyntaxSet, themes: &ThemeSet) -> bool {
        self.reset_numkey_state();
        self.clear_toc_scroll_state();
        let Some(path) = &self.filepath else {
            return false;
        };
        if self.is_diff_mode() {
            return self.reload_diff(ss, themes);
        }
        let Ok(src) = std::fs::read_to_string(path) else {
            return false;
        };
        let file_state = read_file_state(path);
        let content_hash = hash_str(&src);
        self.source = if self.file_mode {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            Self::fence_wrap(&src, ext)
        } else {
            src
        };

        self.reparse_source(ss, themes);
        self.mark_reloaded(file_state, content_hash);
        true
    }

    fn mark_reloaded(&mut self, file_state: Option<FileState>, content_hash: u64) {
        self.last_file_state = file_state;
        self.last_content_hash = content_hash;
        self.last_hash_check = Some(Instant::now());
        if self.watch_flash.is_none() {
            self.reload_flash = Some(Instant::now());
        }
    }

    pub(crate) fn refresh_diff(&mut self, ss: &SyntaxSet, themes: &ThemeSet) -> bool {
        let Some(spec) = self.diff_state.as_ref().map(|state| state.spec.clone()) else {
            return false;
        };
        if matches!(spec.source, super::DiffSource::Patch(_)) {
            return self.reload_diff(ss, themes);
        }
        let Ok(src) = crate::run_git_diff_subprocess(&spec) else {
            return false;
        };
        self.reinstall_diff_keeping_view(&src, spec, ss, themes);
        true
    }

    fn reinstall_diff_keeping_view(
        &mut self,
        src: &str,
        spec: super::DiffSpec,
        ss: &SyntaxSet,
        themes: &ThemeSet,
    ) {
        let view = self.diff_state.as_ref().map(|state| {
            let current = state.clamp_file_idx(state.current_file_idx);
            let (side_is_new, path) = state.files.get(current).map_or((true, String::new()), |f| {
                (
                    !f.new_path.is_empty(),
                    super::diff_preview::current_path(f).to_string(),
                )
            });
            let top = state.first_scroll_target_of_file(current).unwrap_or(0);
            let moved_in_preview =
                state.preview_visible && state.pre_preview_file_idx != Some(current);
            let offset = if moved_in_preview {
                0
            } else {
                state
                    .pre_preview_scroll
                    .unwrap_or(self.scroll)
                    .saturating_sub(top)
            };
            (current, side_is_new, path, offset, state.preview_visible)
        });
        self.install_diff_from_src(src, spec, ss, themes);
        let Some((previous, side_is_new, path, offset, preview_visible)) = view else {
            self.refresh_diff_search();
            return;
        };
        let theme = current_syntect_theme(themes);
        if let Some(state) = self.diff_state.as_mut() {
            let found = if path.is_empty() {
                None
            } else {
                state
                    .files
                    .iter()
                    .position(|f| {
                        let candidate = if side_is_new {
                            &f.new_path
                        } else {
                            &f.old_path
                        };
                        *candidate == path
                    })
                    .or_else(|| {
                        state
                            .files
                            .iter()
                            .position(|f| f.new_path == path || f.old_path == path)
                    })
            };
            let (file, offset) = match found {
                Some(file) => (file, offset),
                None => (state.clamp_file_idx(previous), 0),
            };
            if let Some((top, bottom)) = state.file_bounds(state.layout, file) {
                self.scroll = (top + offset).min(bottom);
            }
            state.current_file_idx = file;
            state.tree_active_idx = file;
            state.nav_pinned_file_idx = Some(file);
        }
        self.clamp_scroll();
        if preview_visible {
            self.diff_toggle_preview(ss, theme);
        }
        self.refresh_diff_search();
    }

    fn reload_diff(&mut self, ss: &SyntaxSet, themes: &ThemeSet) -> bool {
        let Some(path) = self.filepath.clone() else {
            return false;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return false;
        };
        let src = super::mode::decode_lossy(bytes);
        let spec = self
            .diff_state
            .as_ref()
            .map(|state| state.spec.clone())
            .unwrap_or_default();
        self.reinstall_diff_keeping_view(&src, spec, ss, themes);
        self.mark_reloaded(read_file_state(&path), hash_str(&src));
        self.source = src;
        true
    }

    pub(crate) fn load_path(&mut self, path: PathBuf, ss: &SyntaxSet, themes: &ThemeSet) -> bool {
        let Ok(src) = super::mode::read_text_file(&path) else {
            return false;
        };
        let filename = super::path_label(&path);
        let file_state = read_file_state(&path);
        let content_hash = hash_str(&src);
        let detected_mode = super::mode::detect_mode(Some(&path), false, &src);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        let first_load = self.filepath.is_none();
        self.filename = filename;
        if let Some(n) = self.file_history_length.filter(|n| *n > 0) {
            super::history::record_open(path.clone(), n as usize);
        }
        self.filepath = Some(path);
        if first_load && self.watch_from_config {
            self.watch = true;
            self.watch_error = false;
        }
        self.last_file_state = file_state;
        self.last_content_hash = content_hash;
        self.last_hash_check = Some(Instant::now());
        self.reload_flash = None;
        self.scroll = 0;
        self.help_open = false;
        self.file_picker.open = false;
        self.theme_picker.open = false;
        self.search.mode = false;
        self.reset_search_state();
        self.clear_active_goto_line();

        match detected_mode {
            super::AppMode::Diff => {
                let spec = super::DiffSpec::patch(self.filename.clone());
                self.install_diff_from_src(&src, spec, ss, themes);
                self.source = src;
            }
            super::AppMode::Document => {
                self.diff_state = None;
                self.mode = super::AppMode::Document;
                let (wrapped, is_code_file) = Self::wrap_as_code_block(src, &ext, ss);
                self.file_mode = is_code_file;
                self.source = wrapped;
                let theme = current_syntect_theme(themes);
                let at = app_theme();
                let parsed = parse_markdown_with_width(
                    &self.source,
                    ss,
                    theme,
                    self.render_width,
                    &at.markdown,
                    self.file_mode,
                    self.code_line_numbers,
                );
                self.invalidate_theme_previews();
                self.store_current_theme_preview_from(&parsed);
                self.replace_content(parsed);
            }
        }
        self.clear_toc_scroll_state();
        true
    }

    pub(crate) fn reparse_source(&mut self, ss: &SyntaxSet, themes: &ThemeSet) {
        let theme = current_syntect_theme(themes);
        let at = app_theme();
        let old_total = self.total();
        let parsed = parse_markdown_with_width(
            &self.source,
            ss,
            theme,
            self.render_width,
            &at.markdown,
            self.file_mode,
            self.code_line_numbers,
        );
        let new_total = parsed.lines.len();

        if old_total > 0 {
            self.scroll = ((self.scroll as f64 / old_total as f64) * new_total as f64) as usize;
            let vh = self.content_area.height as usize;
            self.scroll = self.scroll.min(new_total.saturating_sub(vh));
        }

        self.invalidate_theme_previews();
        self.store_current_theme_preview_from(&parsed);
        self.replace_content(parsed);
        self.goto_line.target = None;
        self.goto_line.error = false;
        if !self.search.query.is_empty() && !self.search.mode {
            self.run_search();
        }
    }

    pub(crate) fn check_modified(&mut self) -> Option<FileChange> {
        const HASH_FALLBACK_INTERVAL: Duration = Duration::from_secs(2);

        let path = self.filepath.as_ref()?;
        let state = read_file_state(path)?;
        match self.last_file_state {
            Some(prev) if state.modified != prev.modified || state.len != prev.len => {
                Some(FileChange::Metadata(state))
            }
            Some(_) => {
                let should_hash = self
                    .last_hash_check
                    .map(|checked_at| checked_at.elapsed() >= HASH_FALLBACK_INTERVAL)
                    .unwrap_or(true);
                if !should_hash {
                    return None;
                }
                self.last_hash_check = Some(Instant::now());
                let current_hash = hash_file_contents(path).ok()?;
                (current_hash != self.last_content_hash).then_some(FileChange::Content(state))
            }
            None => Some(FileChange::Metadata(state)),
        }
    }

    pub(crate) fn request_reload(&mut self, ss: &SyntaxSet, themes: &ThemeSet) -> bool {
        self.last_file_state = None;
        self.reload(ss, themes)
    }

    pub(crate) fn set_last_content_hash(&mut self, last_content_hash: u64) {
        self.last_content_hash = last_content_hash;
    }

    pub(crate) fn set_last_file_state(&mut self, state: FileState) {
        self.last_file_state = Some(state);
    }

    pub(crate) fn sync_render_width(
        &mut self,
        render_width: usize,
        ss: &SyntaxSet,
        themes: &ThemeSet,
    ) -> bool {
        let next_width = render_width.max(20);
        if self.render_width == next_width {
            return false;
        }
        self.render_width = next_width;

        if matches!(self.mode, crate::app::AppMode::Diff) {
            return true;
        }
        self.reparse_source(ss, themes);
        true
    }
}
