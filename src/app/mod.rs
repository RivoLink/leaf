use crate::{
    markdown::{
        build_searchable_lines,
        toc::{toc_levels, TocEntry},
        CodeBlockInfo, LinkId, LinkSpan,
    },
    render::{build_status_bar, build_toc_line_with_index, toc_header_line},
    theme::{app_theme, current_theme_selection, theme_preset_index},
};
use ratatui::{layout::Rect, text::Line};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub(crate) fn path_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

const PICKER_PAGE_STEP: usize = 5;

fn move_picker_index_page_up(index: &mut usize, total: usize) {
    if total > 0 {
        *index = (*index).saturating_sub(PICKER_PAGE_STEP);
    }
}

fn move_picker_index_page_down(index: &mut usize, total: usize) {
    if total > 0 {
        *index = (*index + PICKER_PAGE_STEP).min(total - 1);
    }
}

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

pub(crate) mod mode;
pub(crate) use mode::{AppMode, DiffSource, DiffSpec, DiffState, PreviewPair};

mod diff_nav;
mod diff_preview;
mod diff_toggles;
pub(crate) use diff_preview::build_preview_pair;
#[cfg(test)]
pub(crate) use diff_preview::{fetch_before, reconstruct_after_from_hunks};

mod flash;
pub(crate) use flash::{
    CodeBlockFlash, DiffFlash, EditorFlash, HistoryFlash, LinkFlash, PathFlash, SoftBreakFlash,
    WatchFlash, COPY_HIGHLIGHT_DURATION_MS, FLASH_DURATION_MS,
};

mod text_copy;
#[cfg(test)]
pub(crate) use text_copy::is_word_char;
#[cfg(test)]
pub(crate) use text_copy::marker_len;
pub(crate) use text_copy::CopyRange;

pub(crate) mod history;
pub(crate) use history::load_history;

mod history_picker;
pub(crate) use history_picker::HistoryPickerState;

mod popups;
pub(crate) use popups::{EditorPickerState, PathKind};

mod links;

mod code_blocks;

mod io_picker;

mod theme_picker;
pub(crate) use theme_picker::ThemePickerState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TocScrollMode {
    Auto(usize),
    Manual(usize),
}

impl TocScrollMode {
    pub(crate) fn offset(self) -> usize {
        match self {
            TocScrollMode::Auto(o) | TocScrollMode::Manual(o) => o,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StatusCacheKey {
    pct: u16,
    link_select: Option<LinkId>,
    terminal_width: u16,
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
    history_flash_active: bool,
    diff_flash_active: bool,
    soft_break_flash_active: bool,
    preview_reconstruct_error: bool,
    mouse_capture: bool,
    toc_scroll_hint_visible: bool,
    diff_tree_scroll_hint_visible: bool,
    diff_sticky_file_idx: Option<usize>,
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
    paragraph_map: Vec<usize>,
    line_number_visible: bool,
    diff_line_numbers: bool,
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
    pub(crate) hovered_diff_tree_idx: Option<usize>,
    toc_scroll_mode: TocScrollMode,
    toc_scroll_hint_dismissed: bool,
    toc_active_pin: Option<usize>,
    status_line: Line<'static>,
    status_cache_key: Option<StatusCacheKey>,
    pub(super) help_open: bool,
    pub(super) path_popup_open: bool,
    pub(super) file_picker: FilePickerState,
    pub(super) history_picker: HistoryPickerState,
    pub(super) pending_picker: PendingPicker,
    pub(super) picker_load_state: PickerLoadState,
    pub(super) file_history_length: Option<i32>,
    pub(super) history_flash: Option<(HistoryFlash, std::time::Instant)>,
    pub(super) diff_flash: Option<(DiffFlash, std::time::Instant)>,
    pub(super) soft_break_flash: Option<(SoftBreakFlash, std::time::Instant)>,
    pub(super) syntax_set: Option<std::sync::Arc<syntect::parsing::SyntaxSet>>,
    pub(super) diff_highlight: Option<(
        std::sync::Arc<syntect::parsing::SyntaxSet>,
        std::sync::Arc<syntect::highlighting::Theme>,
    )>,
    pub(super) history_error_receiver:
        Option<std::sync::mpsc::Receiver<history::HistoryWriteError>>,
    pub(super) history_pending_removals: Vec<std::path::PathBuf>,
    pub(super) theme_picker: ThemePickerState,
    pub(super) editor_picker: EditorPickerState,
    pub(super) render_width: usize,
    pub(crate) content_area: Rect,
    pub(crate) toc_list_area: Option<Rect>,
    pub(crate) diff_tree_list_area: Option<Rect>,
    pub(crate) diff_tree_scroll_hint_dismissed: bool,
    pub(crate) mouse_position: (u16, u16),
    pub(crate) scrollbar_dragging: bool,
    pub(super) editor_config: Option<String>,
    pub(super) editor_flash: Option<(EditorFlash, Instant)>,
    watch_flash: Option<(WatchFlash, Instant)>,
    config_flash: Option<(String, Instant)>,
    pub(crate) link_spans_by_line: HashMap<usize, Vec<LinkSpan>>,
    link_urls: Vec<String>,
    pub(crate) screen_geometry: crate::render::ScreenGeometry,
    screen_geometry_cache: Option<links::ScreenGeometryCacheKey>,
    pub(crate) hovered_link: Option<(usize, usize)>,
    pub(crate) code_blocks: Vec<CodeBlockInfo>,
    pub(crate) code_select: Option<usize>,
    link_select: Option<LinkId>,
    code_block_flash: Option<(CodeBlockFlash, Instant)>,
    link_flash: Option<(LinkFlash, Instant)>,
    path_flash: Option<(PathFlash, Instant)>,
    pub(crate) last_click: Option<(u16, u16, Instant)>,
    pub(super) copy_highlight: Option<(Vec<CopyRange>, Instant)>,
    pub(super) path_copy_flash: Option<(PathKind, bool, Instant)>,
    pub(super) path_popup_hover: Option<PathKind>,
    pub(crate) path_popup_rel_area: Option<Rect>,
    pub(crate) path_popup_abs_area: Option<Rect>,
    numkey_cycle: Option<NumkeyCycleState>,
    reverse_mode: bool,
    pub(super) file_mode: bool,
    pub(super) code_line_numbers: bool,
    keep_soft_breaks_pref: bool,
    soft_breaks_override: Option<bool>,
    pub(super) plain_text_file: bool,
    scroll_with_enter: bool,
    page_scroll_size: crate::config::PageScrollSize,
    max_width: Option<usize>,
    tab_title_max_filename_len: Option<usize>,
    tab_title_length: Option<i32>,
    file_picker_width: crate::picker_width::PickerWidthSpec,
    pub(super) picker_width_floor_active: bool,
    mouse_capture: bool,
    pub(super) mode: AppMode,
    pub(super) diff_state: Option<DiffState>,
    pub(crate) toc_width: crate::picker_width::PickerWidthSpec,
    pub(crate) toc_dragging: bool,
    pub(crate) toc_hover_row: Option<u16>,
    pub(crate) diff_tree_dragging: bool,
    pub(crate) diff_tree_hover_row: Option<u16>,
    pub(crate) terminal_width: u16,
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
            paragraph_map: Vec::new(),
            line_number_visible: false,
            diff_line_numbers: true,
            search: SearchState {
                mode: false,
                draft: String::new(),
                query: String::new(),
                matches: vec![],
                idx: 0,
                draft_hash: 0,
                query_hash: 0,
                layout_switch: None,
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
            hovered_diff_tree_idx: None,
            toc_scroll_mode: TocScrollMode::Auto(0),
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
            history_picker: HistoryPickerState::default(),
            pending_picker: PendingPicker::None,
            picker_load_state: PickerLoadState::Idle,
            file_history_length: None,
            history_flash: None,
            diff_flash: None,
            soft_break_flash: None,
            syntax_set: None,
            diff_highlight: None,
            history_error_receiver: None,
            history_pending_removals: Vec::new(),
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
            diff_tree_list_area: None,
            diff_tree_scroll_hint_dismissed: false,
            mouse_position: (0, 0),
            scrollbar_dragging: false,
            editor_config: None,
            editor_flash: None,
            watch_flash: None,
            config_flash: None,
            link_spans_by_line: HashMap::new(),
            link_urls: Vec::new(),
            screen_geometry: crate::render::ScreenGeometry::default(),
            screen_geometry_cache: None,
            hovered_link: None,
            code_blocks: Vec::new(),
            code_select: None,
            link_select: None,
            code_block_flash: None,
            link_flash: None,
            path_flash: None,
            last_click: None,
            copy_highlight: None,
            path_copy_flash: None,
            path_popup_hover: None,
            path_popup_rel_area: None,
            path_popup_abs_area: None,
            numkey_cycle: None,
            reverse_mode: false,
            file_mode: false,
            code_line_numbers: true,
            keep_soft_breaks_pref: true,
            soft_breaks_override: None,
            plain_text_file: false,
            scroll_with_enter: false,
            page_scroll_size: crate::config::PageScrollSize::default(),
            max_width: None,
            tab_title_max_filename_len: None,
            tab_title_length: None,
            file_picker_width: crate::picker_width::DEFAULT_PICKER_WIDTH,
            picker_width_floor_active: false,
            mouse_capture: true,
            mode: AppMode::Document,
            diff_state: None,
            toc_width: crate::picker_width::DEFAULT_TOC_FILELIST_WIDTH,
            toc_dragging: false,
            toc_hover_row: None,
            diff_tree_dragging: false,
            diff_tree_hover_row: None,
            terminal_width: 80,
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

    pub(crate) fn set_line_numbers_visible(&mut self, value: bool) {
        self.line_number_visible = value;
    }

    pub(crate) fn set_diff_line_numbers(&mut self, value: bool) {
        self.diff_line_numbers = value;
    }

    pub(crate) fn set_code_line_numbers(&mut self, value: bool) {
        self.code_line_numbers = value;
    }

    pub(crate) fn set_keep_soft_breaks(&mut self, value: bool) {
        self.keep_soft_breaks_pref = value;
    }

    pub(crate) fn set_plain_text_file(&mut self, value: bool) {
        self.plain_text_file = value;
    }

    pub(crate) fn default_keep_soft_breaks(pref: bool, plain_text_file: bool) -> bool {
        pref || plain_text_file
    }

    pub(crate) fn keep_soft_breaks(&self) -> bool {
        self.soft_breaks_override
            .unwrap_or(Self::default_keep_soft_breaks(
                self.keep_soft_breaks_pref,
                self.plain_text_file,
            ))
    }

    pub(crate) fn set_scroll_with_enter(&mut self, value: bool) {
        self.scroll_with_enter = value;
    }

    pub(crate) fn set_page_scroll_size(&mut self, value: crate::config::PageScrollSize) {
        self.page_scroll_size = value;
    }

    pub(crate) fn set_max_width(&mut self, max_width: Option<usize>) {
        self.max_width = max_width;
    }

    pub(crate) fn set_tab_title_max_filename_len(&mut self, value: Option<usize>) {
        self.tab_title_max_filename_len = value;
    }

    pub(crate) fn tab_title_max_filename_len(&self) -> Option<usize> {
        self.tab_title_max_filename_len
    }

    pub(crate) fn set_tab_title_length(&mut self, value: Option<i32>) {
        self.tab_title_length = value;
    }

    pub(crate) fn tab_title_length(&self) -> Option<i32> {
        self.tab_title_length
    }

    pub(crate) fn set_file_picker_width(&mut self, value: crate::picker_width::PickerWidthSpec) {
        self.file_picker_width = value;
    }

    pub(crate) fn set_toc_width(&mut self, value: crate::picker_width::PickerWidthSpec) {
        self.toc_width = value;
    }

    fn max_toc_content_width(area_width: u16) -> u16 {
        (area_width / 2).saturating_sub(1)
    }

    fn toc_content_width(&self, area_width: u16) -> u16 {
        let raw = crate::picker_width::picker_width_raw(self.toc_width, area_width);
        clamp_toc_content(raw, Self::max_toc_content_width(area_width))
    }

    pub(crate) fn toc_width_cells(&self, area_width: u16) -> u16 {
        self.toc_content_width(area_width) + 1
    }

    pub(crate) fn grow_toc_width(&mut self) {
        let area_width = self.terminal_width;
        let max_content = Self::max_toc_content_width(area_width);
        let new = (self.toc_content_width(area_width) + 5).min(max_content);
        self.set_toc_content_width(new);
    }

    pub(crate) fn shrink_toc_width(&mut self) {
        let area_width = self.terminal_width;
        let new = self
            .toc_content_width(area_width)
            .saturating_sub(5)
            .max(crate::picker_width::TOC_WIDTH_MIN);
        self.set_toc_content_width(new);
    }

    pub(crate) fn set_toc_width_cells(&mut self, total_cells: u16) {
        let max_content = Self::max_toc_content_width(self.terminal_width);
        self.set_toc_content_width(clamp_toc_content(
            total_cells.saturating_sub(1),
            max_content,
        ));
    }

    fn set_toc_content_width(&mut self, width: u16) {
        self.toc_width = crate::picker_width::PickerWidthSpec::Cells(width);
        self.invalidate_toc_display();
    }

    fn invalidate_toc_display(&mut self) {
        self.toc_display_lines.clear();
    }

    pub(crate) fn set_file_history_length(&mut self, value: Option<i32>) {
        self.file_history_length = value;
    }

    pub(crate) fn file_history_length(&self) -> Option<i32> {
        self.file_history_length
    }

    pub(crate) fn set_history_error_receiver(
        &mut self,
        receiver: std::sync::mpsc::Receiver<history::HistoryWriteError>,
    ) {
        self.history_error_receiver = Some(receiver);
    }

    pub(crate) fn poll_history_errors(&mut self) -> bool {
        let received = if let Some(ref rx) = self.history_error_receiver {
            let mut any = false;
            while rx.try_recv().is_ok() {
                any = true;
            }
            any
        } else {
            false
        };
        if received {
            self.set_history_flash(HistoryFlash::WriteFailed);
        }
        received
    }

    pub(crate) fn is_mouse_capture_enabled(&self) -> bool {
        self.mouse_capture
    }

    pub(crate) fn toggle_mouse_capture(&mut self) -> bool {
        self.mouse_capture = !self.mouse_capture;
        if !self.mouse_capture {
            self.hovered_link = None;
            self.hovered_toc_idx = None;
            self.hovered_diff_tree_idx = None;
            self.scrollbar_dragging = false;
            self.toc_dragging = false;
            self.toc_hover_row = None;
            self.diff_tree_dragging = false;
            self.diff_tree_hover_row = None;
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

    pub(crate) fn scroll_with_enter(&self) -> bool {
        self.scroll_with_enter
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
        paragraph_map: Vec<usize>,
    ) {
        self.line_number_map = line_number_map;
        self.source_line_map = source_line_map;
        self.paragraph_map = paragraph_map;
    }

    pub(crate) fn paragraph_at(&self, idx: usize) -> usize {
        self.paragraph_map.get(idx).copied().unwrap_or(0)
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

    pub(crate) fn title_filename(&self) -> Option<&str> {
        self.has_content().then_some(self.filename.as_str())
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
            self.adjust_auto_toc_offset();
            return;
        }

        self.toc_active_idx = active_idx;
        let mut top_level_index = 0usize;
        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut entries: Vec<usize> = Vec::new();
        let mut active_display_idx = None;
        let panel_width = self.toc_width_cells(self.terminal_width);
        for (idx, entry, display_level) in self.visible_toc_entries() {
            let is_active = active_idx == Some(idx);
            let line = build_toc_line_with_index(
                entry,
                display_level,
                (display_level == 1).then_some(top_level_index),
                is_active,
                panel_width,
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
        self.adjust_auto_toc_offset();
    }

    fn adjust_auto_toc_offset(&mut self) {
        let TocScrollMode::Auto(current_offset) = self.toc_scroll_mode else {
            return;
        };
        let (Some(active), Some(area)) = (self.toc_active_display_idx, self.toc_list_area) else {
            return;
        };
        let list_height = area.height as usize;
        if list_height == 0 {
            return;
        }
        let max_offset = self.max_toc_scroll_offset(area.height);

        let new_offset = if active + 1 >= current_offset + list_height {
            (active + 2).saturating_sub(list_height)
        } else if active <= current_offset {
            active.saturating_sub(1)
        } else {
            current_offset
        };
        self.toc_scroll_mode = TocScrollMode::Auto(new_offset.min(max_offset));
    }

    pub(crate) fn refresh_status_cache(&mut self, pct: u16) {
        let cache_key = StatusCacheKey {
            pct,
            link_select: self.link_select,
            terminal_width: self.terminal_width,
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
            history_flash_active: self
                .history_flash
                .as_ref()
                .map(|(_, t)| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            diff_flash_active: self
                .diff_flash
                .as_ref()
                .map(|(_, t)| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            soft_break_flash_active: self
                .soft_break_flash
                .as_ref()
                .map(|(_, t)| t.elapsed() < Duration::from_millis(FLASH_DURATION_MS))
                .unwrap_or(false),
            preview_reconstruct_error: self.is_diff_preview_reconstruction_error(),
            mouse_capture: self.mouse_capture,
            toc_scroll_hint_visible: self.is_toc_scroll_hint_visible(),
            diff_tree_scroll_hint_visible: self.is_diff_tree_scroll_hint_visible(),
            diff_sticky_file_idx: self.diff_current_file_from_scroll(),
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

    pub(crate) fn mode(&self) -> AppMode {
        self.mode
    }

    pub(crate) fn set_mode(&mut self, mode: AppMode) {
        self.mode = mode;
        self.copy_highlight = None;
    }

    pub(crate) fn diff(&self) -> Option<&DiffState> {
        self.diff_state.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn diff_mut(&mut self) -> Option<&mut DiffState> {
        self.diff_state.as_mut()
    }

    pub(crate) fn is_diff_preview_visible(&self) -> bool {
        self.diff_state.as_ref().is_some_and(|s| s.preview_visible)
    }

    pub(crate) fn is_diff_preview_reconstruction_error(&self) -> bool {
        let Some(state) = self.diff_state.as_ref() else {
            return false;
        };
        if !state.preview_visible || state.files.is_empty() {
            return false;
        }
        let active_idx = state
            .tree_active_idx
            .min(state.files.len().saturating_sub(1));
        state
            .preview_cache
            .get(&active_idx)
            .is_some_and(|p| p.after_error.is_some())
    }

    pub(crate) fn is_diff_mode(&self) -> bool {
        self.mode == AppMode::Diff
    }

    #[cfg(test)]
    pub(crate) fn set_source_for_test(&mut self, src: String) {
        self.source = src;
    }

    pub(crate) fn set_syntax_set(&mut self, ss: std::sync::Arc<syntect::parsing::SyntaxSet>) {
        self.syntax_set = Some(ss);
    }

    fn shared_syntax_set(
        &self,
        ss: &syntect::parsing::SyntaxSet,
    ) -> std::sync::Arc<syntect::parsing::SyntaxSet> {
        match &self.syntax_set {
            Some(shared) => std::sync::Arc::clone(shared),
            None => std::sync::Arc::new(ss.clone()),
        }
    }

    pub(crate) fn install_diff_from_src(
        &mut self,
        src: &str,
        spec: DiffSpec,
        ss: &syntect::parsing::SyntaxSet,
        themes: &syntect::highlighting::ThemeSet,
    ) {
        let files = crate::diff::parse_unified_diff(src);
        let ss_arc = self.shared_syntax_set(ss);
        let theme_arc = std::sync::Arc::new(crate::theme::current_syntect_theme(themes).clone());
        self.install_diff(files, spec, ss_arc, theme_arc);
    }

    pub(crate) fn install_diff(
        &mut self,
        files: Vec<crate::diff::DiffFile>,
        spec: DiffSpec,
        ss: std::sync::Arc<syntect::parsing::SyntaxSet>,
        theme: std::sync::Arc<syntect::highlighting::Theme>,
    ) {
        self.mode = AppMode::Diff;
        let carry_tree_visible = self.diff_state.as_ref().is_some_and(|s| s.tree_visible);
        let carry_layout = self
            .diff_state
            .as_ref()
            .map(|s| s.layout)
            .unwrap_or_default();
        let built = crate::render::build_diff_lines_with(&files, &ss, &theme, false);
        let files_arc = std::sync::Arc::new(files);
        let mut state = DiffState::new(files_arc.clone(), spec);
        let lines = apply_unified_build(&mut state, built);
        state.tree_visible = carry_tree_visible;
        state.layout = carry_layout;
        state.split_ready = false;
        state.preview_cache = std::collections::HashMap::new();
        state.preview_visible = false;
        state.pre_preview_scroll = None;
        state.pre_preview_file_idx = None;
        state.preview_scroll_cache = std::collections::HashMap::new();
        state.nav_pinned_file_idx = None;
        self.diff_state = Some(state);
        self.search.layout_switch = None;
        self.diff_tree_scroll_hint_dismissed = false;
        self.replace_content(diff_parse_result(lines));
        self.scroll = 0;
        self.line_number_visible = self.diff_line_numbers;
        if matches!(carry_layout, crate::diff::DiffLayout::Split) {
            self.build_split_now(&ss, &theme);
        }
        self.diff_highlight = Some((ss, theme));
    }

    pub(crate) fn highlight_visible_diff_files(&mut self, viewport_rows: usize) {
        let scroll = self.scroll;
        let Some(state) = self.diff_state.as_mut() else {
            return;
        };
        if state.preview_visible {
            return;
        }
        let layout = state.layout;
        let split = matches!(layout, crate::diff::DiffLayout::Split);
        if split && !state.split_ready {
            return;
        }
        let mut pending: Vec<usize> = {
            let owner = state.file_owner(layout);
            let flags = if split {
                &state.split_highlighted_files
            } else {
                &state.highlighted_files
            };
            let end = scroll.saturating_add(viewport_rows).min(owner.len());
            owner[scroll.min(end)..end]
                .iter()
                .flatten()
                .copied()
                .filter(|&fi| !flags.get(fi).copied().unwrap_or(true))
                .collect()
        };
        if pending.is_empty() {
            return;
        }
        let Some((ss, theme)) = self.diff_highlight.clone() else {
            return;
        };
        pending.dedup();
        for fi in pending {
            let Some((top, bottom)) = state.file_bounds(layout, fi) else {
                continue;
            };
            let file = &state.files[fi..=fi];
            let (mut rows, file_bottom) = if split {
                let built = crate::render::build_split_lines_with(file, &ss, &theme, true);
                (built.lines, built.file_bottom_idx[0])
            } else {
                let built = crate::render::build_diff_lines_with(file, &ss, &theme, true);
                (built.lines, built.file_bottom_idx[0])
            };
            rows.truncate(file_bottom + 1);
            let (target, flags) = if split {
                (&mut state.split_lines, &mut state.split_highlighted_files)
            } else {
                (&mut self.lines, &mut state.highlighted_files)
            };
            debug_assert_eq!(rows.len(), bottom - top + 1, "highlighted rows must match");
            if rows.len() == bottom - top + 1 {
                for (dst, row) in target[top..=bottom].iter_mut().zip(rows) {
                    *dst = row;
                }
            }
            flags[fi] = true;
        }
        self.highlighted_line_cache = None;
    }
}

impl App {
    pub(crate) fn rebuild_diff_view(
        &mut self,
        ss: &syntect::parsing::SyntaxSet,
        theme: &syntect::highlighting::Theme,
    ) {
        let Some(state) = self.diff_state.as_ref() else {
            return;
        };
        let files = std::sync::Arc::clone(&state.files);
        let built = crate::render::build_diff_lines_with(&files, ss, theme, false);
        let ss_arc = self.shared_syntax_set(ss);
        self.diff_highlight = Some((ss_arc, std::sync::Arc::new(theme.clone())));
        let state = self.diff_state.as_mut().expect("checked");
        let lines = apply_unified_build(state, built);
        state.split_ready = false;
        let preview_visible = state.preview_visible;
        let split = matches!(state.layout, crate::diff::DiffLayout::Split);
        let current = state.clamp_file_idx(state.current_file_idx);
        state.preview_cache.clear();
        let preview_input = preview_visible.then(|| {
            let active = state.clamp_file_idx(state.tree_active_idx);
            (active, state.spec.clone())
        });
        self.replace_content(diff_parse_result(lines));

        if split {
            self.build_split_now(ss, theme);
        }
        if let Some((active, spec)) = preview_input.filter(|(active, _)| *active < files.len()) {
            let pair = crate::app::build_preview_pair(&files[active], &spec, ss, theme);
            if let Some(state) = self.diff_state.as_mut() {
                state.preview_cache.insert(active, pair);
            }
        }
        if !preview_visible {
            if let Some(state) = self.diff_state.as_mut() {
                state.nav_pinned_file_idx = Some(current);
            }
        }
        self.clamp_scroll();
        self.refresh_diff_search();
    }
}

fn clamp_toc_content(width: u16, max_content: u16) -> u16 {
    width.clamp(
        crate::picker_width::TOC_WIDTH_MIN.min(max_content),
        max_content,
    )
}

fn diff_parse_result(lines: Vec<Line<'static>>) -> crate::markdown::ParseResult {
    crate::markdown::ParseResult {
        lines,
        ..Default::default()
    }
}

fn apply_unified_build(
    state: &mut DiffState,
    built: crate::render::BuiltDiffLines,
) -> Vec<Line<'static>> {
    state.line_old_num = built.line_old_num;
    state.line_new_num = built.line_new_num;
    state.file_header_status = built.file_header_status;
    state.file_owner = built.file_owner;
    state.file_top_idx = built.file_top_idx;
    state.file_bottom_idx = built.file_bottom_idx;
    state.meta_only_bottom_rows = built.meta_only_bottom_rows;
    state.highlighted_files = vec![false; state.file_top_idx.len()];
    state.search_lines = diff_search_lines(&built.lines);
    built.lines
}

fn diff_search_lines(lines: &[Line<'static>]) -> Vec<String> {
    let is_decoration = |c: char| ('\u{2500}'..='\u{257f}').contains(&c);
    lines
        .iter()
        .map(|line| {
            let mut text = String::new();
            let mut after_border = false;
            for span in &line.spans {
                let content = span.content.as_ref();
                if !content.is_empty() && content.chars().all(is_decoration) {
                    after_border = true;
                    continue;
                }
                let sigil = after_border && matches!(content, "+" | "-" | " ");
                after_border = false;
                if !sigil {
                    text.push_str(content);
                }
            }
            if text.is_ascii() {
                text.make_ascii_lowercase();
                text
            } else {
                text.to_lowercase()
            }
        })
        .collect()
}

pub(crate) fn apply_split_build(state: &mut DiffState, built: crate::render::BuiltSplitLines) {
    let split_search_lines = diff_search_lines(&built.lines);
    state.split_lines = built.lines;
    state.split_line_old_num_left = built.line_old_num_left;
    state.split_line_new_num_left = built.line_new_num_left;
    state.split_line_old_num_right = built.line_old_num_right;
    state.split_line_new_num_right = built.line_new_num_right;
    state.split_file_header_status = built.file_header_status;
    state.split_file_owner = built.file_owner;
    state.split_file_top_idx = built.file_top_idx;
    state.split_file_bottom_idx = built.file_bottom_idx;
    state.split_meta_only_frame_rows = built.meta_only_frame_rows;
    state.split_single_column_body_rows = built.single_column_body_rows;
    state.split_combined_rows = built.combined_rows;
    state.split_left_gutter_disabled = built.left_gutter_disabled;
    state.split_right_gutter_disabled = built.right_gutter_disabled;
    state.split_search_lines = split_search_lines;
    state.split_highlighted_files = vec![false; state.split_file_top_idx.len()];
    state.split_ready = true;
}

impl App {
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
