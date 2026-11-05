use super::{App, LinkFlash};
use crate::markdown::{
    is_footnote_number, is_ordered_marker, width::rendered_cluster_width, ALERT_ICONS,
    BLOCKQUOTE_BAR, BULLET_DEEP, BULLET_L1, BULLET_L2, TASK_CHECKED, TASK_CHECKED_ALT,
    TASK_UNCHECKED,
};
use crate::theme::app_theme;
use ratatui::text::Line;
use unicode_segmentation::UnicodeSegmentation;

const BOX_DRAWING: &[&str] = &[
    "┌", "┐", "└", "┘", "├", "┤", "┬", "┴", "┼", "─", "│", "╞", "╡", "╪", "═", "╔", "╗", "╚", "╝",
    "║",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CopyRange {
    pub(crate) line_idx: usize,
    pub(crate) start_col: usize,
    pub(crate) end_col: usize,
}

pub(crate) struct CopyTarget {
    pub(crate) text: String,
    pub(crate) ranges: Vec<CopyRange>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Text,
    Marker,
    Footnote,
    Padding,
    Border,
}

struct Cluster<'a> {
    symbol: &'a str,
    start_col: usize,
    end_col: usize,
    role: Role,
    chip: bool,
}

pub(crate) fn is_word_char(prev: Option<char>, c: char, next: Option<char>) -> bool {
    let is_core = |c: char| c.is_alphanumeric() || c == '_';
    is_core(c)
        || (matches!(c, '\'' | '’' | '-') && prev.is_some_and(is_core) && next.is_some_and(is_core))
}

fn is_decoration(symbol: &str) -> bool {
    [
        BULLET_L1,
        BULLET_L2,
        BULLET_DEEP,
        TASK_CHECKED,
        TASK_CHECKED_ALT,
        TASK_UNCHECKED,
        BLOCKQUOTE_BAR,
    ]
    .iter()
    .any(|marker| marker.trim_end() == symbol)
        || BOX_DRAWING.contains(&symbol)
}

fn is_strippable(cluster: &Cluster<'_>) -> bool {
    match cluster.role {
        Role::Padding => false,
        Role::Marker | Role::Footnote | Role::Border => true,
        Role::Text => cluster.symbol.trim().is_empty() || is_decoration(cluster.symbol),
    }
}

fn marker_len(content: &str) -> usize {
    if is_ordered_marker(content) {
        return usize::MAX;
    }
    ALERT_ICONS
        .iter()
        .find(|icon| {
            content
                .strip_prefix(**icon)
                .is_some_and(|rest| rest.starts_with(' '))
        })
        .map_or(0, |icon| icon.len() + 1)
}

fn line_clusters<'a>(line: &'a Line<'_>) -> Vec<Cluster<'a>> {
    let theme = app_theme().markdown;
    let chip_bgs = [theme.inline_code_bg, theme.latex_inline_bg, theme.mark_bg];
    let mut clusters = Vec::new();
    let mut col = 0;
    for span in &line.spans {
        let footnote = span.style.fg == Some(theme.footnote_ref)
            && is_footnote_number(span.content.trim_end());
        let markers = marker_len(&span.content);
        let border = span.content == "│";
        let chip = span.style.bg.is_some_and(|bg| chip_bgs.contains(&bg));
        for (idx, symbol) in span.content.graphemes(true).enumerate() {
            let start_col = col;
            col += rendered_cluster_width(symbol);
            clusters.push(Cluster {
                symbol,
                start_col,
                end_col: col,
                role: if border {
                    Role::Border
                } else if footnote {
                    Role::Footnote
                } else if idx < markers {
                    Role::Marker
                } else {
                    Role::Text
                },
                chip,
            });
        }
    }

    for run in clusters
        .chunk_by_mut(|a, b| a.chip == b.chip)
        .filter(|run| run[0].chip)
    {
        let last = run.len() - 1;
        for edge in [0, last] {
            if run[edge].symbol == " " {
                run[edge].role = Role::Padding;
            }
        }
    }
    clusters
}

fn border_positions(clusters: &[Cluster<'_>]) -> Vec<usize> {
    clusters
        .iter()
        .enumerate()
        .filter(|(_, c)| c.role == Role::Border)
        .map(|(idx, _)| idx)
        .collect()
}

fn table_cell_clusters(clusters: Vec<Cluster<'_>>, cell: usize) -> Option<Vec<Cluster<'_>>> {
    let borders = border_positions(&clusters);
    let (&left, &right) = (borders.get(cell)?, borders.get(cell + 1)?);
    Some(
        clusters
            .into_iter()
            .skip(left + 1)
            .take(right - left - 1)
            .collect(),
    )
}

impl App {
    fn table_cell_at(&self, col: u16, row: u16, line_idx: usize) -> Option<Option<usize>> {
        let clusters = line_clusters(&self.lines[line_idx]);
        let borders = border_positions(&clusters);
        if borders.is_empty() {
            return Some(None);
        }
        let (column, row) = self.screen_cell(col, row)?;
        let line_col = (*self.screen_geometry.rows.get(row)?.get(column)?)?;
        let hit = clusters
            .iter()
            .find(|c| c.start_col <= line_col && line_col < c.end_col)?;
        if hit.role == Role::Border {
            return None;
        }
        let passed = borders
            .iter()
            .filter(|&&idx| clusters[idx].end_col <= line_col)
            .count();
        (passed > 0 && passed < borders.len()).then(|| Some(passed - 1))
    }

    pub(crate) fn word_at(&self, col: u16, row: u16) -> Option<CopyTarget> {
        let (column, row) = self.screen_cell(col, row)?;
        let line_idx = *self.screen_geometry.row_lines.get(row)?;
        let line_col = (*self.screen_geometry.rows.get(row)?.get(column)?)?;
        let clusters = line_clusters(&self.lines[line_idx]);
        let hit = clusters
            .iter()
            .position(|c| c.start_col <= line_col && line_col < c.end_col)?;

        let char_at = |idx: usize| {
            clusters
                .get(idx)
                .filter(|c| c.role != Role::Footnote)
                .and_then(|c| c.symbol.chars().next())
        };
        let is_word = |idx: usize| {
            char_at(idx).is_some_and(|c| {
                is_word_char(idx.checked_sub(1).and_then(char_at), c, char_at(idx + 1))
            })
        };
        if !is_word(hit) {
            return None;
        }
        let mut first = hit;
        while first > 0 && is_word(first - 1) {
            first -= 1;
        }
        let mut last = hit;
        while is_word(last + 1) {
            last += 1;
        }

        let is_padding = |idx: usize| clusters.get(idx).is_some_and(|c| c.role == Role::Padding);
        let start = if first > 0 && is_padding(first - 1) {
            first - 1
        } else {
            first
        };
        let end = if is_padding(last + 1) { last + 1 } else { last };
        Some(CopyTarget {
            text: clusters[first..=last].iter().map(|c| c.symbol).collect(),
            ranges: vec![CopyRange {
                line_idx,
                start_col: clusters[start].start_col,
                end_col: clusters[end].end_col,
            }],
        })
    }

    pub(crate) fn logical_line_at(&self, col: u16, row: u16) -> Option<CopyTarget> {
        let line_idx = self.line_idx_at(col, row)?;
        let number = self.line_number_at(line_idx);
        let cell = self.table_cell_at(col, row, line_idx)?;
        let mut start = line_idx;
        while start > 0 && self.line_number_at(start - 1) == number {
            start -= 1;
        }
        let mut end = line_idx;
        while end + 1 < self.lines.len() && self.line_number_at(end + 1) == number {
            end += 1;
        }

        let mut parts = Vec::new();
        let mut ranges = Vec::new();
        for idx in start..=end {
            let clusters = match cell {
                Some(cell) => match table_cell_clusters(line_clusters(&self.lines[idx]), cell) {
                    Some(clusters) => clusters,
                    None => continue,
                },
                None => line_clusters(&self.lines[idx]),
            };
            let Some(first) = clusters.iter().position(|c| !is_strippable(c)) else {
                continue;
            };
            let last = clusters.iter().rposition(|c| !is_strippable(c)).unwrap();
            let kept = &clusters[first..=last];
            let text: String = kept
                .iter()
                .filter(|c| c.role == Role::Text)
                .map(|c| c.symbol)
                .collect();
            if text.is_empty() {
                continue;
            }
            parts.push(text);
            ranges.push(CopyRange {
                line_idx: idx,
                start_col: clusters[first].start_col,
                end_col: clusters[last].end_col,
            });
        }
        if parts.is_empty() {
            return None;
        }
        Some(CopyTarget {
            text: parts.join(" "),
            ranges,
        })
    }

    pub(crate) fn code_line_at(&self, block_idx: usize, line_idx: usize) -> Option<CopyTarget> {
        let block = self.code_blocks.get(block_idx)?;
        let first_row = block.rendered_start + 1;
        let line = block.code_rows.get(line_idx.checked_sub(first_row)?)?.line;
        let text = block.raw_content.lines().nth(line)?;
        let ranges = block
            .code_rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.line == line)
            .map(|(offset, row)| CopyRange {
                line_idx: first_row + offset,
                start_col: row.start_col,
                end_col: row.end_col,
            })
            .collect();
        Some(CopyTarget {
            text: text.to_string(),
            ranges,
        })
    }

    pub(crate) fn copy_text_with(&mut self, target: CopyTarget, copy: impl FnOnce(&str) -> bool) {
        if copy(&target.text) {
            self.set_link_flash(LinkFlash::Copied);
            self.set_copy_highlight(target.ranges);
        } else {
            self.set_link_flash(LinkFlash::CopyFailed);
        }
    }
}
