use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
    pub(crate) first_change_row: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DiffState {
    pub(crate) files: Arc<Vec<DiffFile>>,
    pub(crate) spec: DiffSpec,
    pub(crate) split_ready: bool,
    pub(crate) current_file_idx: usize,
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
    pub(crate) split_combined_rows: Vec<usize>,
    pub(crate) split_left_gutter_disabled: Vec<bool>,
    pub(crate) split_right_gutter_disabled: Vec<bool>,
    pub(crate) search_lines: Vec<String>,
    pub(crate) split_search_lines: Vec<String>,
    pub(crate) highlighted_files: Vec<bool>,
    pub(crate) split_highlighted_files: Vec<bool>,
    pub(crate) file_change_counts: Vec<(usize, usize)>,
    pub(crate) change_totals: (usize, usize),
    pub(crate) gutter_widths: (usize, usize),
    pub(crate) nav_pinned_file_idx: Option<usize>,
    pub(crate) preview_visible: bool,
    pub(crate) preview_cache: HashMap<usize, PreviewPair>,
    pub(crate) pre_preview_scroll: Option<usize>,
    pub(crate) pre_preview_file_idx: Option<usize>,
    pub(crate) preview_scroll_cache: HashMap<usize, usize>,
}

impl DiffState {
    pub(crate) fn new(files: Arc<Vec<DiffFile>>, spec: DiffSpec) -> Self {
        let file_change_counts: Vec<(usize, usize)> = files
            .iter()
            .map(crate::render::count_file_changes)
            .collect();
        let change_totals = file_change_counts
            .iter()
            .fold((0, 0), |(a, d), (fa, fd)| (a + fa, d + fd));
        let gutter_widths = gutter_digit_widths(&files);
        Self {
            files,
            spec,
            split_ready: false,
            current_file_idx: 0,
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
            split_combined_rows: Vec::new(),
            split_left_gutter_disabled: Vec::new(),
            split_right_gutter_disabled: Vec::new(),
            search_lines: Vec::new(),
            split_search_lines: Vec::new(),
            highlighted_files: Vec::new(),
            split_highlighted_files: Vec::new(),
            file_change_counts,
            change_totals,
            gutter_widths,
            nav_pinned_file_idx: None,
            preview_visible: false,
            preview_cache: HashMap::new(),
            pre_preview_scroll: None,
            pre_preview_file_idx: None,
            preview_scroll_cache: HashMap::new(),
        }
    }

    pub(crate) fn totals(&self) -> (usize, usize) {
        self.change_totals
    }

    pub(crate) fn clamp_file_idx(&self, idx: usize) -> usize {
        idx.min(self.files.len().saturating_sub(1))
    }

    pub(crate) fn first_scroll_target_of_file(&self, file_idx: usize) -> Option<usize> {
        self.file_bounds(self.layout, file_idx).map(|(top, _)| top)
    }

    pub(crate) fn convert_scroll(&self, from: DiffLayout, to: DiffLayout, row: usize) -> usize {
        let Some(file) = self.row_file(from, row) else {
            return 0;
        };
        let Some((top, bottom)) = self.file_bounds(to, file) else {
            return row;
        };
        let (old, new) = self.row_line_numbers(from, row);
        if old == 0 && new == 0 {
            return top;
        }
        self.find_numbered_row(to, (top, bottom), (old, new), |_| true)
            .unwrap_or(top)
    }

    fn find_numbered_row(
        &self,
        layout: DiffLayout,
        (top, bottom): (usize, usize),
        (old, new): (u32, u32),
        accept: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        (top..=bottom).find(|&r| {
            let (o, n) = self.row_line_numbers(layout, r);
            ((old != 0 && o == old) || (new != 0 && n == new)) && accept(r)
        })
    }

    pub(crate) fn convert_match_row(
        &self,
        from: DiffLayout,
        to: DiffLayout,
        row: usize,
        matches: &[usize],
    ) -> Option<usize> {
        if matches.is_empty() {
            return None;
        }
        let is_match = |r: usize| matches.binary_search(&r).is_ok();
        let (old, new) = self.row_line_numbers(from, row);
        let file_bounds = self
            .row_file(from, row)
            .and_then(|file| Some((file, self.file_bounds(to, file)?)));
        let converted = match file_bounds {
            None => self.convert_scroll(from, to, row),
            Some((file, (top, bottom))) if self.is_combined_file(file) => {
                let from_top = self.file_bounds(from, file).map_or(row, |(t, _)| t);
                (top + row.saturating_sub(from_top)).min(bottom)
            }
            Some((file, bounds)) if old == 0 && new == 0 => {
                self.convert_unnumbered_row(from, to, row, file, bounds)
            }
            Some((_, bounds)) => self
                .find_numbered_row(to, bounds, (old, new), is_match)
                .unwrap_or_else(|| self.convert_scroll(from, to, row)),
        };
        let nearest = |rows: &mut dyn Iterator<Item = usize>| {
            rows.min_by_key(|&m| (m.abs_diff(converted), m < converted))
        };
        file_bounds
            .and_then(|(_, (top, bottom))| {
                nearest(&mut matches.iter().copied().filter(|&m| m >= top && m <= bottom))
            })
            .or_else(|| nearest(&mut matches.iter().copied()))
    }

    fn convert_unnumbered_row(
        &self,
        from: DiffLayout,
        to: DiffLayout,
        row: usize,
        file: usize,
        (to_top, to_bottom): (usize, usize),
    ) -> usize {
        let Some((from_top, from_bottom)) = self.file_bounds(from, file) else {
            return to_top;
        };
        let anchor = |r: usize| {
            let numbers = self.row_line_numbers(from, r);
            if numbers == (0, 0) {
                return None;
            }
            self.find_numbered_row(to, (to_top, to_bottom), numbers, |_| true)
        };
        if let Some((next, at)) = (row + 1..=from_bottom).find_map(|r| Some((r, anchor(r)?))) {
            return at
                .saturating_sub(next - row)
                .clamp(to_top, to_bottom.max(to_top));
        }
        if let Some((prev, at)) = (from_top..row).rev().find_map(|r| Some((r, anchor(r)?))) {
            return (at + (row - prev)).min(to_bottom);
        }
        (to_top + row.saturating_sub(from_top)).min(to_bottom)
    }

    fn is_combined_file(&self, file: usize) -> bool {
        self.files
            .get(file)
            .is_some_and(|f| f.hunks.iter().any(|h| h.header.parent_count >= 2))
    }

    pub(super) fn file_owner(&self, layout: DiffLayout) -> &[Option<usize>] {
        match layout {
            DiffLayout::Unified => &self.file_owner,
            DiffLayout::Split => &self.split_file_owner,
        }
    }

    pub(super) fn row_file(&self, layout: DiffLayout, row: usize) -> Option<usize> {
        let owner = self.file_owner(layout);
        owner
            .get(row)
            .copied()
            .flatten()
            .or_else(|| {
                owner[..row.min(owner.len())]
                    .iter()
                    .rev()
                    .flatten()
                    .next()
                    .copied()
            })
            .or_else(|| owner.iter().skip(row).flatten().next().copied())
    }

    pub(super) fn file_bounds(&self, layout: DiffLayout, file: usize) -> Option<(usize, usize)> {
        let (top, bottom) = match layout {
            DiffLayout::Unified => (self.file_top_idx.get(file), self.file_bottom_idx.get(file)),
            DiffLayout::Split => (
                self.split_file_top_idx.get(file),
                self.split_file_bottom_idx.get(file),
            ),
        };
        Some((*top?, *bottom?))
    }

    fn row_line_numbers(&self, layout: DiffLayout, row: usize) -> (u32, u32) {
        let get = |nums: &[u32]| nums.get(row).copied().unwrap_or(0);
        match layout {
            DiffLayout::Unified => (get(&self.line_old_num), get(&self.line_new_num)),
            DiffLayout::Split => (
                get(&self.split_line_old_num_left),
                get(&self.split_line_new_num_right),
            ),
        }
    }

    pub(crate) fn file_counts(&self, file_idx: usize) -> (usize, usize) {
        self.file_change_counts
            .get(file_idx)
            .copied()
            .unwrap_or_default()
    }
}

fn gutter_digit_widths(files: &[DiffFile]) -> (usize, usize) {
    let max_end = |start_count: fn(&crate::diff::HunkHeader) -> u32| {
        files
            .iter()
            .flat_map(|f| f.hunks.iter())
            .map(|h| start_count(&h.header))
            .max()
            .unwrap_or(0)
    };
    let max_old = max_end(|h| h.old_start.saturating_add(h.old_count));
    let max_new = max_end(|h| h.new_start.saturating_add(h.new_count));
    (
        max_old.max(1).to_string().len(),
        max_new.max(1).to_string().len(),
    )
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
    Revs(String, String),
    Patch(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DiffSpec {
    pub(crate) source: DiffSource,
    pub(crate) paths: Vec<String>,
    // Where Git runs; `None` inherits the process working directory.
    pub(crate) cwd: Option<PathBuf>,
}

impl DiffSpec {
    pub(crate) fn git_command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new("git");
        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }
        cmd
    }

    pub(crate) fn patch(label: String) -> Self {
        Self {
            source: DiffSource::Patch(label),
            ..Self::default()
        }
    }
}

fn is_git_file_header(line: &[u8]) -> bool {
    crate::diff::GIT_FILE_HEADER_PREFIXES
        .iter()
        .any(|prefix| line.starts_with(prefix.as_bytes()))
}

const DIFF_EXTENSIONS: &[&str] = &["diff", "patch"];

pub(crate) fn decode_lossy(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes)
        .unwrap_or_else(|err| String::from_utf8_lossy(err.as_bytes()).into_owned())
}

pub(crate) fn read_text_file(path: &std::path::Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let is_diff_file = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(is_diff_extension);
    match String::from_utf8(bytes) {
        Ok(src) => Ok(src),
        Err(err) if is_diff_file => Ok(decode_lossy(err.into_bytes())),
        Err(err) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
    }
}

pub(crate) fn is_diff_extension(ext: &str) -> bool {
    let lower = ext.to_ascii_lowercase();
    DIFF_EXTENSIONS.iter().any(|candidate| *candidate == lower)
}

pub(crate) fn detect_mode_from_bytes(buf: &[u8]) -> AppMode {
    const SCANNED_LINES: usize = 64;
    let lines: Vec<&[u8]> = buf
        .split(|&b| b == b'\n')
        .take(SCANNED_LINES)
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .collect();
    if detect_mode_from_lines(lines.iter().copied()) == AppMode::Diff {
        return AppMode::Diff;
    }
    let mut fence: Option<(u8, usize)> = None;
    for (i, line) in lines.iter().enumerate() {
        let indent = line.iter().take_while(|b| **b == b' ').count();
        let trimmed = &line[indent..];
        let fence_char = trimmed
            .first()
            .copied()
            .filter(|c| indent <= 3 && (*c == b'`' || *c == b'~'));
        let run = fence_char.map_or(0, |c| trimmed.iter().take_while(|b| **b == c).count());
        match (fence, fence_char) {
            (None, Some(c)) if run >= 3 => {
                fence = Some((c, run));
                continue;
            }
            (Some((open, len)), Some(c))
                if c == open && run >= len && trimmed[run..].trim_ascii().is_empty() =>
            {
                fence = None;
                continue;
            }
            _ => {}
        }
        if fence.is_some() {
            continue;
        }
        let next = |k: usize| lines.get(i + k).copied().unwrap_or_default();
        let diff_header = is_git_file_header(line);
        let svn_header = line.starts_with(b"Index: ")
            && next(1).starts_with(b"====")
            && next(2).starts_with(b"--- ");
        let unified_header = line.starts_with(b"--- ")
            && next(1).starts_with(b"+++ ")
            && next(2).starts_with(b"@@ ");
        if diff_header || svn_header || unified_header {
            return AppMode::Diff;
        }
    }
    AppMode::Document
}

pub(crate) fn detect_mode_from_content(src: &str) -> AppMode {
    detect_mode_from_lines(src.split('\n').map(str::as_bytes))
}

fn detect_mode_from_lines<'a>(mut lines: impl Iterator<Item = &'a [u8]>) -> AppMode {
    for _ in 0..16 {
        let Some(line) = lines.next() else {
            break;
        };
        if line.iter().all(|c| c.is_ascii_whitespace()) {
            continue;
        }
        let next = lines.next().unwrap_or_default();
        return if starts_diff(line, next) {
            AppMode::Diff
        } else {
            AppMode::Document
        };
    }
    AppMode::Document
}

fn starts_diff(line: &[u8], next: &[u8]) -> bool {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    if is_git_file_header(line) {
        return true;
    }
    if let Some(rest) = line.strip_prefix(b"commit ") {
        let sha_len = rest.iter().take_while(|c| c.is_ascii_hexdigit()).count();
        return matches!(sha_len, 40 | 64) && sha_len == rest.len();
    }
    if let Some(rest) = line.strip_prefix(b"From ") {
        let sha_len = rest.iter().take_while(|c| c.is_ascii_hexdigit()).count();
        return sha_len >= 40 && rest.get(sha_len) == Some(&b' ');
    }
    line.starts_with(b"--- ") && next.starts_with(b"+++ ")
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
            if super::App::is_markdown_extension(ext) || ext.eq_ignore_ascii_case("mdx") {
                return AppMode::Document;
            }
        }
    }
    detect_mode_from_content(src)
}
