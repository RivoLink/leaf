use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use ratatui::text::{Line, Span};

use crate::diff::{DiffFile, DiffLayout};

#[derive(Clone, Debug, Default)]
pub(crate) struct PreviewPair {
    pub(crate) before_lines: Vec<Line<'static>>,
    pub(crate) after_lines: Vec<Line<'static>>,
    pub(crate) before_is_note: bool,
    pub(crate) after_is_note: bool,
    pub(crate) after_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HunkRange {
    pub(crate) file_idx: usize,
    pub(crate) hunk_idx: usize,
    pub(crate) header_line: usize,
    pub(crate) body_start: usize,
    pub(crate) body_end: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DiffState {
    pub(crate) files: Arc<Vec<DiffFile>>,
    pub(crate) spec: DiffSpec,
    pub(crate) split_ready: bool,
    pub(crate) current_file_idx: usize,
    pub(crate) hunk_ranges: Vec<HunkRange>,
    pub(crate) line_owner: Vec<Option<(usize, usize)>>,
    pub(crate) line_old_num: Vec<u32>,
    pub(crate) line_new_num: Vec<u32>,
    pub(crate) file_header_status: Vec<Option<Vec<Span<'static>>>>,
    pub(crate) file_owner: Vec<Option<usize>>,
    pub(crate) file_top_idx: Vec<usize>,
    pub(crate) file_bottom_idx: Vec<usize>,
    pub(crate) meta_only_bottom_rows: Vec<usize>,
    pub(crate) tree_visible: bool,
    pub(crate) tree_active_idx: usize,
    pub(crate) tree_scroll: usize,
    pub(crate) tree_scroll_manual: bool,
    pub(crate) layout: DiffLayout,
    pub(crate) split_lines: Vec<ratatui::text::Line<'static>>,
    pub(crate) split_line_old_num_left: Vec<u32>,
    pub(crate) split_line_new_num_left: Vec<u32>,
    pub(crate) split_line_old_num_right: Vec<u32>,
    pub(crate) split_line_new_num_right: Vec<u32>,
    pub(crate) split_file_header_status: Vec<Option<Vec<Span<'static>>>>,
    pub(crate) split_file_owner: Vec<Option<usize>>,
    pub(crate) split_file_top_idx: Vec<usize>,
    pub(crate) split_file_bottom_idx: Vec<usize>,
    pub(crate) split_meta_only_frame_rows: Vec<usize>,
    pub(crate) split_single_column_body_rows: Vec<usize>,
    pub(crate) split_left_gutter_disabled: Vec<bool>,
    pub(crate) split_right_gutter_disabled: Vec<bool>,
    pub(crate) nav_pinned_file_idx: Option<usize>,
    pub(crate) preview_visible: bool,
    pub(crate) preview_cache: HashMap<usize, PreviewPair>,
    pub(crate) pre_preview_scroll: Option<usize>,
    pub(crate) pre_preview_file_idx: Option<usize>,
    pub(crate) preview_scroll_cache: HashMap<usize, usize>,
}

impl DiffState {
    pub(crate) fn new(files: Arc<Vec<DiffFile>>, spec: DiffSpec) -> Self {
        Self {
            files,
            spec,
            split_ready: false,
            current_file_idx: 0,
            hunk_ranges: Vec::new(),
            line_owner: Vec::new(),
            line_old_num: Vec::new(),
            line_new_num: Vec::new(),
            file_header_status: Vec::new(),
            file_owner: Vec::new(),
            file_top_idx: Vec::new(),
            file_bottom_idx: Vec::new(),
            meta_only_bottom_rows: Vec::new(),
            tree_visible: false,
            tree_active_idx: 0,
            tree_scroll: 0,
            tree_scroll_manual: false,
            layout: DiffLayout::default(),
            split_lines: Vec::new(),
            split_line_old_num_left: Vec::new(),
            split_line_new_num_left: Vec::new(),
            split_line_old_num_right: Vec::new(),
            split_line_new_num_right: Vec::new(),
            split_file_header_status: Vec::new(),
            split_file_owner: Vec::new(),
            split_file_top_idx: Vec::new(),
            split_file_bottom_idx: Vec::new(),
            split_meta_only_frame_rows: Vec::new(),
            split_single_column_body_rows: Vec::new(),
            split_left_gutter_disabled: Vec::new(),
            split_right_gutter_disabled: Vec::new(),
            nav_pinned_file_idx: None,
            preview_visible: false,
            preview_cache: HashMap::new(),
            pre_preview_scroll: None,
            pre_preview_file_idx: None,
            preview_scroll_cache: HashMap::new(),
        }
    }

    pub(crate) fn totals(&self) -> (usize, usize) {
        let mut adds = 0usize;
        let mut dels = 0usize;
        for f in self.files.iter() {
            for h in &f.hunks {
                for l in &h.lines {
                    match l {
                        crate::diff::HunkLine::Add(_) => adds += 1,
                        crate::diff::HunkLine::Del(_) => dels += 1,
                        _ => {}
                    }
                }
            }
        }
        (adds, dels)
    }

    pub(crate) fn first_scroll_target_of_file(&self, file_idx: usize) -> Option<usize> {
        match self.layout {
            DiffLayout::Unified => self.file_top_idx.get(file_idx).copied(),
            DiffLayout::Split => self.split_file_top_idx.get(file_idx).copied(),
        }
    }

    pub(crate) fn file_counts(&self, file_idx: usize) -> (usize, usize) {
        let mut added = 0usize;
        let mut deleted = 0usize;
        if let Some(file) = self.files.get(file_idx) {
            for h in &file.hunks {
                for l in &h.lines {
                    match l {
                        crate::diff::HunkLine::Add(_) => added += 1,
                        crate::diff::HunkLine::Del(_) => deleted += 1,
                        _ => {}
                    }
                }
            }
        }
        (added, deleted)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AppMode {
    #[default]
    Document,
    Diff,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum DiffSource {
    #[default]
    Working,
    Cached,
    Ref(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DiffSpec {
    pub(crate) source: DiffSource,
}

const DIFF_PREFIXES: &[&[u8]] = &[b"diff --git ", b"diff --cc ", b"diff --combined "];

const DIFF_EXTENSIONS: &[&str] = &["diff", "patch"];

pub(crate) fn is_diff_extension(ext: &str) -> bool {
    let lower = ext.to_ascii_lowercase();
    DIFF_EXTENSIONS.iter().any(|candidate| *candidate == lower)
}

pub(crate) fn detect_mode_from_bytes(buf: &[u8]) -> AppMode {
    let mut lines_seen = 0usize;
    let mut line_start = 0usize;
    for (i, &b) in buf.iter().enumerate() {
        if b == b'\n' {
            let line = &buf[line_start..i];
            if !line.iter().all(|c| c.is_ascii_whitespace()) {
                return match_bytes_prefix(line);
            }
            lines_seen += 1;
            if lines_seen >= 16 {
                return AppMode::Document;
            }
            line_start = i + 1;
        }
    }

    if line_start < buf.len() {
        let line = &buf[line_start..];
        if !line.iter().all(|c| c.is_ascii_whitespace()) {
            return match_bytes_prefix(line);
        }
    }
    AppMode::Document
}

fn match_bytes_prefix(line: &[u8]) -> AppMode {
    for prefix in DIFF_PREFIXES {
        if line.starts_with(prefix) {
            return AppMode::Diff;
        }
    }
    AppMode::Document
}

pub(crate) fn detect_mode_from_content(src: &str) -> AppMode {
    src.lines()
        .take(16)
        .find(|l| !l.trim().is_empty())
        .map_or(AppMode::Document, |line| {
            match_bytes_prefix(line.as_bytes())
        })
}

pub(crate) fn detect_mode(path: Option<&Path>, cli_forced: bool, src: &str) -> AppMode {
    if cli_forced {
        return AppMode::Diff;
    }
    if let Some(p) = path {
        if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            if is_diff_extension(ext) {
                return AppMode::Diff;
            }
        }
    }
    detect_mode_from_content(src)
}
