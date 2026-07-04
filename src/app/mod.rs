use crate::{
    markdown::{
        build_searchable_lines,
        toc::{toc_levels, TocEntry},
        CodeBlockInfo, LinkSpan,
    },
    render::{build_status_bar, build_toc_line_with_index, toc_header_line},
    theme::{app_theme, current_theme_selection, theme_preset_index},
};
use ratatui::{layout::Rect, text::Line};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

mod search;
pub(crate) use search::SearchState;

mod goto_line;
pub(crate) use goto_line::GotoLineState;

mod file_picker;
mod fuzzy;
pub(crate) use file_picker::{FilePickerMode, FilePickerState, PickerIndexTruncation};
use file_picker::{PendingPicker, PickerLoadState};

mod navigation;
use navigation::NumkeyCycleState;

mod content;
pub(crate) use content::{FileChange, FileState};

mod flash;
pub(crate) use flash::{
    CodeBlockFlash, EditorFlash, LinkFlash, PathFlash, WatchFlash, FLASH_DURATION_MS,
};

mod popups;
pub(crate) use popups::{EditorPickerState, PathKind};

mod links;

mod code_blocks;

mod io_picker;

mod theme_picker;
pub(crate) use theme_picker::ThemePickerState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TocScrollMode {
    Auto,
    Manual(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StatusCacheKey {
    pct: u16,
    search_mode: bool,
    search_draft_hash: u64,
    search_query_hash: u64,
    search_draft_len: usize,
    search_query_len: usize,
    search_match_count: usize,
    search_idx: usize,
    goto_line_mode: bool,
    goto_line_draft_hash: u64,
    goto_line_target: Option<usize>,
    goto_line_error: bool,
    watch: bool,
    flash_active: bool,
    editor_flash_active: bool,
    file_picker_open: bool,
    picker_loading: bool,
    watch_flash_active: bool,
    watch_error: bool,
    config_flash_active: bool,
    link_flash_active: bool,
    path_flash_active: bool,
    code_block_flash_active: bool,
    mouse_capture: bool,
    toc_scroll_hint_visible: bool,
}

pub(crate) struct AppConfig {
    pub(crate) filename: String,
    pub(crate) source: String,
    pub(crate) debug_input: bool,
    pub(crate) watch: bool,
    pub(crate) filepath: Option<PathBuf>,
    pub(crate) last_file_state: Option<FileState>,
}

pub(crate) struct App {
    pub(super) lines: Vec<Line<'static>>,
    pub(super) plain_lines: Vec<String>,
    pub(super) scroll: usize,
    pub(super) toc: Vec<TocEntry>,
    toc_visible: bool,
    line_number_map: Vec<usize>,
    source_line_map: Vec<usize>,
    line_number_visible: bool,
    pub(super) search: SearchState,
    pub(super) goto_line: GotoLineState,
    pub(super) debug_input: bool,
    pub(super) filename: String,
    pub(super) source: String,
    watch: bool,
    watch_from_config: bool,
    watch_error: bool,
    pub(super) filepath: Option<PathBuf>,
    dir_arg: Option<PathBuf>,
    pub(super) last_file_state: Option<FileState>,
    pub(super) last_content_hash: u64,
    pub(super) last_hash_check: Option<Instant>,
    pub(super) reload_flash: Option<Instant>,
    highlighted_line_cache: Option<(usize, u64, Line<'static>)>,
    toc_display_lines: Vec<Line<'static>>,
    toc_display_entries: Vec<usize>,
    toc_header_line: Line<'static>,
    pub(crate) toc_active_idx: Option<usize>,
    pub(crate) toc_active_display_idx: Option<usize>,
    pub(crate) hovered_toc_idx: Option<usize>,
    toc_scroll_mode: TocScrollMode,
    toc_scroll_hint_dismissed: bool,
    toc_active_pin: Option<usize>,
    status_line: Line<'static>,
    status_cache_key: Option<StatusCacheKey>,
    pub(super) help_open: bool,
    pub(super) path_popup_open: bool,
    pub(super) file_picker: FilePickerState,
    pub(super) pending_picker: PendingPicker,
    pub(super) picker_load_state: PickerLoadState,
    pub(super) theme_picker: ThemePickerState,
    pub(super) editor_picker: EditorPickerState,
    pub(super) render_width: usize,
    pub(crate) content_area: Rect,
    pub(crate) toc_list_area: Option<Rect>,
    pub(crate) mouse_position: (u16, u16),
    pub(crate) scrollbar_dragging: bool,
    pub(super) editor_config: Option<String>,
    pub(super) editor_flash: Option<(EditorFlash, Instant)>,
    watch_flash: Option<(WatchFlash, Instant)>,
    config_flash: Option<(String, Instant)>,
    pub(crate) link_spans_by_line: HashMap<usize, Vec<LinkSpan>>,
    pub(crate) hovered_link: Option<(usize, usize)>,
    pub(crate) code_blocks: Vec<CodeBlockInfo>,
    pub(crate) code_select: Option<usize>,
    code_block_flash: Option<(CodeBlockFlash, Instant)>,
    link_flash: Option<(LinkFlash, Instant)>,
    path_flash: Option<(PathFlash, Instant)>,
    pub(crate) last_click: Option<(u16, u16, Instant)>,
    pub(super) path_copy_flash: Option<(PathKind, bool, Instant)>,
    pub(super) path_popup_hover: Option<PathKind>,
    pub(crate) path_popup_rel_area: Option<Rect>,
    pub(crate) path_popup_abs_area: Option<Rect>,
    numkey_cycle: Option<NumkeyCycleState>,
    reverse_mode: bool,
    pub(super) file_mode: bool,
    pub(super) code_line_numbers: bool,
    max_width: Option<usize>,
    mouse_capture: bool,
}

impl App {
    #[cfg(test)]
    pub(crate) fn new(
        lines: Vec<Line<'static>>,
        toc: Vec<TocEntry>,
        filename: String,
        debug_input: bool,
        watch: bool,
        filepath: Option<PathBuf>,
        last_file_state: Option<FileState>,
    ) -> Self {
        let source = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        Self::new_with_source(
            lines,
            toc,
            AppConfig {
                filename,
                source,
                debug_input,
                watch,
                filepath,
                last_file_state,
            },
        )
    }

    pub(crate) fn new_with_source(
        lines: Vec<Line<'static>>,
        toc: Vec<TocEntry>,
        config: AppConfig,
    ) -> Self {
        let AppConfig {
            filename,
            source,
            debug_input,
            watch,
            filepath,
            last_file_state,
        } = config;
        let plain_lines = build_searchable_lines(&lines)
            .into_iter()
            .map(|line| line.to_lowercase())
            .collect();
        let mut app = Self {
            lines,
            plain_lines,
            scroll: 0,
            toc,
            toc_visible: false,
            line_number_map: Vec::new(),
            source_line_map: Vec::new(),
            line_number_visible: false,
            search: SearchState {
                mode: false,
                draft: String::new(),
                query: String::new(),
                matches: vec![],
                idx: 0,
                draft_hash: 0,
                query_hash: 0,
            },
            goto_line: GotoLineState {
                mode: false,
                draft: String::new(),
                target: None,
                error: false,
                draft_hash: 0,
            },
            debug_input,
            filename,
            source,
            watch,
            watch_from_config: false,
            watch_error: false,
            filepath,
            dir_arg: None,
            last_file_state,
            last_content_hash: 0,
            last_hash_check: None,
            reload_flash: None,
            highlighted_line_cache: None,
            toc_display_lines: Vec::new(),
            toc_display_entries: Vec::new(),
            toc_header_line: toc_header_line(),
            toc_active_idx: None,
            toc_active_display_idx: None,
            hovered_toc_idx: None,
            toc_scroll_mode: TocScrollMode::Auto,
            toc_scroll_hint_dismissed: false,
            toc_active_pin: None,
            status_line: Line::default(),
            status_cache_key: None,
            help_open: false,
            path_popup_open: false,
            file_picker: FilePickerState {
                open: false,
                mode: FilePickerMode::Browser,
                dir: PathBuf::from("."),
                extras: Vec::new(),
                entries: Vec::new(),
                filtered: Vec::new(),
                match_positions: Vec::new(),
                index: 0,
                query: String::new(),
                truncation: None,
            },
            pending_picker: PendingPicker::None,
            picker_load_state: PickerLoadState::Idle,
            theme_picker: ThemePickerState {
                open: false,
                index: theme_preset_index(current_theme_selection().preset_hint()),
                original: None,
                original_preview: None,
                preview_cache: vec![None; crate::theme::THEME_PRESETS.len()],
            },
            editor_picker: EditorPickerState {
                open: false,
                editors: Vec::new(),
                index: 0,
            },
            render_width: 80,
            content_area: Rect::default(),
            toc_list_area: None,
            mouse_position: (0, 0),
            scrollbar_dragging: false,
            editor_config: None,
            editor_flash: None,
            watch_flash: None,
            config_flash: None,
            link_spans_by_line: HashMap::new(),
            hovered_link: None,
            code_blocks: Vec::new(),
            code_select: None,
            code_block_flash: None,
            link_flash: None,
            path_flash: None,
            last_click: None,
            path_copy_flash: None,
            path_popup_hover: None,
            path_popup_rel_area: None,
            path_popup_abs_area: None,
            numkey_cycle: None,
            reverse_mode: false,
            file_mode: false,
            code_line_numbers: true,
            max_width: None,
            mouse_capture: true,
        };
        app.store_current_theme_preview();
        app.refresh_static_caches();
        app
    }

    pub(crate) fn set_extras(&mut self, extras: Vec<String>) {
        self.file_picker.extras = extras;
    }

    pub(crate) fn set_file_mode(&mut self, file_mode: bool) {
        self.file_mode = file_mode;
    }

    pub(crate) fn set_code_line_numbers(&mut self, value: bool) {
        self.code_line_numbers = value;
    }

    pub(crate) fn set_max_width(&mut self, max_width: Option<usize>) {
        self.max_width = max_width;
    }

    pub(crate) fn is_mouse_capture_enabled(&self) -> bool {
        self.mouse_capture
    }

    pub(crate) fn toggle_mouse_capture(&mut self) -> bool {
        self.mouse_capture = !self.mouse_capture;
        if !self.mouse_capture {
            self.hovered_link = None;
            self.hovered_toc_idx = None;
            self.scrollbar_dragging = false;
        }
        self.mouse_capture
    }

    pub(crate) fn max_width(&self) -> Option<usize> {
        self.max_width
    }

    pub(crate) fn set_watch_from_config(&mut self, value: bool) {
        self.watch_from_config = value;
    }

    pub(crate) fn is_watch_enabled(&self) -> bool {
        self.watch
    }

    pub(crate) fn is_watch_error(&self) -> bool {
        self.watch_error
    }

    pub(crate) fn set_watch_error(&mut self, error: bool) {
        self.watch_error = error;
    }

    pub(crate) fn debug_input_enabled(&self) -> bool {
        self.debug_input
    }

    pub(crate) fn is_toc_visible(&self) -> bool {
        self.toc_visible
    }

    pub(crate) fn is_line_number_visible(&self) -> bool {
        self.line_number_visible
    }

    pub(crate) fn line_number_total(&self) -> usize {
        self.line_number_map.last().copied().unwrap_or(0)
    }

    pub(crate) fn line_number_at(&self, idx: usize) -> usize {
        self.line_number_map.get(idx).copied().unwrap_or(0)
    }

    pub(crate) fn line_number_gutter_width(&self) -> usize {
        if self.line_number_visible {
            self.line_number_total().max(1).to_string().len() + 2
        } else {
            0
        }
    }

    pub(crate) fn set_line_maps(
        &mut self,
        line_number_map: Vec<usize>,
        source_line_map: Vec<usize>,
    ) {
        self.line_number_map = line_number_map;
        self.source_line_map = source_line_map;
    }

    pub(crate) fn source_line_at(&self, idx: usize) -> usize {
        self.source_line_map.get(idx).copied().unwrap_or(1)
    }

    pub(crate) fn has_toc(&self) -> bool {
        !self.toc.is_empty()
    }

    // Always >= 5 (scroll padding).
    // Use has_content() to check for actual content.
    pub(crate) fn total(&self) -> usize {
        self.lines.len()
    }

    pub(crate) fn scroll(&self) -> usize {
        self.scroll
    }

    pub(crate) fn visible_lines(&self, start: usize, end: usize) -> &[Line<'static>] {
        &self.lines[start..end]
    }

    pub(crate) fn highlighted_line_cache(&self) -> Option<(usize, &Line<'static>)> {
        self.highlighted_line_cache
            .as_ref()
            .map(|(idx, _, line)| (*idx, line))
    }

    pub(crate) fn toc_display_lines(&self) -> &[Line<'static>] {
        &self.toc_display_lines
    }

    pub(crate) fn toc_display_entries(&self) -> &[usize] {
        &self.toc_display_entries
    }

    pub(super) fn visible_toc_entries(&self) -> impl Iterator<Item = (usize, &TocEntry, u8)> {
        let levels = toc_levels(&self.toc);
        self.toc.iter().enumerate().filter_map(move |(idx, entry)| {
            let display_level = levels.as_ref()?.display_level(entry.level)?;
            Some((idx, entry, display_level))
        })
    }

    pub(crate) fn toc_header_line(&self) -> &Line<'static> {
        &self.toc_header_line
    }

    pub(crate) fn status_line(&self) -> &Line<'static> {
        &self.status_line
    }

    pub(crate) fn filename(&self) -> &str {
        &self.filename
    }

    #[cfg(test)]
    pub(crate) fn line(&self, idx: usize) -> Option<&Line<'static>> {
        self.lines.get(idx)
    }

    pub(crate) fn active_toc_index(&self) -> Option<usize> {
        if let Some(pin) = self.toc_active_pin {
            return Some(pin);
        }
        let mut first_visible = None;
        let mut active = None;
        let mut last_visible = None;
        for (idx, entry, _) in self.visible_toc_entries() {
            if first_visible.is_none() {
                first_visible = Some((idx, entry.line));
            }
            last_visible = Some(idx);
            if entry.line <= self.scroll {
                active = Some(idx);
            }
        }

        let max = self.max_scroll();
        if max > 0 && self.scroll >= max {
            active = last_visible;
        }

        let (first_idx, first_line) = first_visible?;
        if self.scroll < first_line {
            Some(first_idx)
        } else {
            active.or(Some(first_idx))
        }
    }

    pub(crate) fn refresh_highlighted_line_cache(&mut self, line_idx: usize) -> Option<()> {
        let qh = self.search.query_hash;
        let needs_refresh = self
            .highlighted_line_cache
            .as_ref()
            .map(|(idx, hash, _)| *idx != line_idx || *hash != qh)
            .unwrap_or(true);
        if needs_refresh {
            let line = self.lines.get(line_idx)?;
            let theme = app_theme();
            self.highlighted_line_cache = Some((
                line_idx,
                qh,
                crate::markdown::highlight_line(line, &theme.markdown, &self.search.query),
            ));
        }
        Some(())
    }

    pub(crate) fn refresh_toc_cache(&mut self) {
        let active_idx = self.active_toc_index();
        if self.toc_active_idx == active_idx && !self.toc_display_lines.is_empty() {
            return;
        }

        self.toc_active_idx = active_idx;
        let mut top_level_index = 0usize;
        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut entries: Vec<usize> = Vec::new();
        let mut active_display_idx = None;
        for (idx, entry, display_level) in self.visible_toc_entries() {
            let is_active = active_idx == Some(idx);
            let line = build_toc_line_with_index(
                entry,
                display_level,
                (display_level == 1).then_some(top_level_index),
                is_active,
            );
            if display_level == 1 {
                top_level_index += 1;
            }
            if is_active {
                active_display_idx = Some(entries.len());
            }
            lines.push(line);
            entries.push(idx);
        }
        debug_assert_eq!(lines.len(), entries.len());
        self.toc_display_lines = lines;
        self.toc_display_entries = entries;
        self.toc_active_display_idx = active_display_idx;
    }

    pub(crate) fn refresh_status_cache(&mut self, pct: u16) {
        let cache_key = StatusCacheKey {
            pct,
            search_mode: self.search.mode,
            search_draft_hash: self.search.draft_hash,
            search_query_hash: self.search.query_hash,
            search_draft_len: self.search.draft.len(),
            search_query_len: self.search.query.len(),
            search_match_count: self.search.matches.len(),
            search_idx: self.search.idx,
            goto_line_mode: self.goto_line.mode,
            goto_line_draft_hash: self.goto_line.draft_hash,
            goto_line_target: self.goto_line.target,
            goto_line_error: self.goto_line.error,
            watch: self.watch,
            flash_active: self
                .reload_flash
                .map(|t| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            editor_flash_active: self
                .editor_flash
                .as_ref()
                .map(|(_, t)| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            file_picker_open: self.is_file_picker_open(),
            picker_loading: self.is_picker_loading(),
            watch_flash_active: self
                .watch_flash
                .as_ref()
                .map(|(_, t)| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            watch_error: self.watch_error,
            config_flash_active: self
                .config_flash
                .as_ref()
                .map(|(_, t)| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            link_flash_active: self
                .link_flash
                .as_ref()
                .map(|(_, t)| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            path_flash_active: self
                .path_flash
                .as_ref()
                .map(|(_, t)| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            code_block_flash_active: self
                .code_block_flash
                .as_ref()
                .map(|(_, t)| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            mouse_capture: self.mouse_capture,
            toc_scroll_hint_visible: self.is_toc_scroll_hint_visible(),
        };

        if self.status_cache_key.as_ref() == Some(&cache_key) {
            return;
        }

        self.status_line = Line::from(build_status_bar(self, pct));
        self.status_cache_key = Some(cache_key);
    }

    pub(crate) fn refresh_static_caches(&mut self) {
        self.toc_active_idx = None;
        self.toc_active_display_idx = None;
        self.toc_active_pin = None;
        self.toc_display_lines.clear();
        self.toc_display_entries.clear();
        self.refresh_toc_cache();
        self.status_cache_key = None;
    }

    pub(crate) fn set_editor_config(&mut self, editor: Option<String>) {
        self.editor_config = editor;
    }

    pub(crate) fn editor_config(&self) -> Option<&str> {
        self.editor_config.as_deref()
    }

    pub(crate) fn filepath(&self) -> Option<&std::path::Path> {
        self.filepath.as_deref()
    }

    pub(crate) fn has_content(&self) -> bool {
        self.filepath.is_some() || !self.source.is_empty()
    }

    pub(crate) fn set_dir_arg(&mut self, dir: PathBuf) {
        self.dir_arg = Some(dir);
    }

    pub(crate) fn picker_dir(&self) -> PathBuf {
        if let Some(ref dir) = self.dir_arg {
            return dir.clone();
        }
        std::env::current_dir()
            .ok()
            .or_else(|| {
                self.filepath
                    .as_ref()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            })
            .unwrap_or_default()
    }
}
