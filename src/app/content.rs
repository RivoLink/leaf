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
            line_number_map,
            source_line_map,
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
        self.link_spans_by_line = super::links::link_spans_to_map(link_spans);
        self.hovered_link = None;
        self.set_code_blocks(code_blocks);
        self.code_select = None;
        self.set_line_maps(line_number_map, source_line_map);
        self.refresh_static_caches();
    }

    pub(crate) fn reload(&mut self, ss: &SyntaxSet, themes: &ThemeSet) -> bool {
        self.reset_numkey_state();
        self.clear_toc_scroll_state();
        let path = match &self.filepath {
            Some(p) => p,
            None => return false,
        };
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return false,
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
        self.last_file_state = file_state;
        self.last_content_hash = content_hash;
        self.last_hash_check = Some(Instant::now());
        if self.watch_flash.is_none() {
            self.reload_flash = Some(Instant::now());
        }
        true
    }

    pub(crate) fn load_path(&mut self, path: PathBuf, ss: &SyntaxSet, themes: &ThemeSet) -> bool {
        let src = match std::fs::read_to_string(&path) {
            Ok(src) => src,
            Err(_) => return false,
        };
        let filename = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let file_state = read_file_state(&path);
        let content_hash = hash_str(&src);
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let (src, is_code_file) = Self::wrap_as_code_block(src, ext, ss);
        self.file_mode = is_code_file;
        let theme = current_syntect_theme(themes);
        let at = app_theme();
        let parsed = parse_markdown_with_width(
            &src,
            ss,
            theme,
            self.render_width,
            &at.markdown,
            self.file_mode,
            self.code_line_numbers,
        );

        let first_load = self.filepath.is_none();
        self.filename = filename;
        self.source = src;
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
        self.invalidate_theme_preview_cache();
        self.store_current_theme_preview_from(&parsed.lines, &parsed.toc);
        self.replace_content(parsed);
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

        self.invalidate_theme_preview_cache();
        self.store_current_theme_preview_from(&parsed.lines, &parsed.toc);
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
        self.reparse_source(ss, themes);
        true
    }
}
