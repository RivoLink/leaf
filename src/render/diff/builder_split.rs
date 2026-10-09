use super::builder_common::{
    assert_sidecars_parallel, build_combined_half, combined_row_bg, SYNTECT_SKIP_HUNK_LINES,
};
use super::*;
use crate::{
    diff::{DiffFile, FileChange, HunkLine, NO_NEWLINE_MARKER},
    theme::app_theme,
};
use ratatui::{
    style::Style,
    text::{Line, Span},
};
use syntect::{
    easy::HighlightLines,
    highlighting::Theme as SyntectTheme,
    parsing::{SyntaxReference, SyntaxSet},
};

#[derive(Clone, Debug, Default)]
pub(crate) struct BuiltSplitLines {
    pub(crate) lines: Vec<Line<'static>>,
    pub(crate) line_old_num_left: Vec<u32>,
    pub(crate) line_new_num_left: Vec<u32>,
    pub(crate) line_old_num_right: Vec<u32>,
    pub(crate) line_new_num_right: Vec<u32>,
    pub(crate) file_header_status: Vec<Option<Vec<Span<'static>>>>,
    pub(crate) file_owner: Vec<Option<usize>>,
    pub(crate) file_top_idx: Vec<usize>,
    pub(crate) file_bottom_idx: Vec<usize>,
    pub(crate) meta_only_frame_rows: Vec<usize>,
    pub(crate) single_column_body_rows: Vec<usize>,
    pub(crate) left_gutter_disabled: Vec<bool>,
    pub(crate) right_gutter_disabled: Vec<bool>,
}

pub(super) const SPLIT_COL_CONTENT_BUDGET: usize = 512;

pub(crate) fn build_split_lines(
    files: &[DiffFile],
    ss: &SyntaxSet,
    theme_syntect: &SyntectTheme,
) -> BuiltSplitLines {
    let theme = app_theme();
    let diff = &theme.diff;

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut line_old_num_left: Vec<u32> = Vec::new();
    let mut line_new_num_left: Vec<u32> = Vec::new();
    let mut line_old_num_right: Vec<u32> = Vec::new();
    let mut line_new_num_right: Vec<u32> = Vec::new();
    let mut file_header_status: Vec<Option<Vec<Span<'static>>>> = Vec::new();
    let mut file_owner: Vec<Option<usize>> = Vec::new();
    let mut file_top_idx: Vec<usize> = Vec::new();
    let mut file_bottom_idx: Vec<usize> = Vec::new();
    let mut meta_only_frame_rows: Vec<usize> = Vec::new();
    let mut single_column_body_rows: Vec<usize> = Vec::new();

    let push = |lines: &mut Vec<Line<'static>>,
                line_old_num_left: &mut Vec<u32>,
                line_new_num_left: &mut Vec<u32>,
                line_old_num_right: &mut Vec<u32>,
                line_new_num_right: &mut Vec<u32>,
                file_header_status: &mut Vec<Option<Vec<Span<'static>>>>,
                line: Line<'static>,
                ol: u32,
                nl: u32,
                or: u32,
                nr: u32,
                status: Option<Vec<Span<'static>>>| {
        lines.push(line);
        line_old_num_left.push(ol);
        line_new_num_left.push(nl);
        line_old_num_right.push(or);
        line_new_num_right.push(nr);
        file_header_status.push(status);
    };

    if files.is_empty() {
        push(
            &mut lines,
            &mut line_old_num_left,
            &mut line_new_num_left,
            &mut line_old_num_right,
            &mut line_new_num_right,
            &mut file_header_status,
            Line::from(Span::styled(
                "No changes to display".to_string(),
                Style::default().fg(diff.context_fg),
            )),
            0,
            0,
            0,
            0,
            None,
        );
        file_owner.push(None);
        return BuiltSplitLines {
            lines,
            line_old_num_left,
            line_new_num_left,
            line_old_num_right,
            line_new_num_right,
            file_header_status,
            file_owner,
            file_top_idx,
            file_bottom_idx,
            meta_only_frame_rows,
            single_column_body_rows,
            left_gutter_disabled: Vec::new(),
            right_gutter_disabled: Vec::new(),
        };
    }

    for (fi, file) in files.iter().enumerate() {
        let syntax: Option<&SyntaxReference> = resolve_file_syntax(file, ss);

        let mut hl_left = syntax.map(|s| HighlightLines::new(s, theme_syntect));
        let mut hl_right = syntax.map(|s| HighlightLines::new(s, theme_syntect));

        let (file_add, file_del) = count_file_changes(file);
        let (top_line, status_spans) = build_file_top_frame_parts(file, diff, file_add, file_del);
        let top_line_idx = lines.len();
        file_top_idx.push(top_line_idx);
        push(
            &mut lines,
            &mut line_old_num_left,
            &mut line_new_num_left,
            &mut line_old_num_right,
            &mut line_new_num_right,
            &mut file_header_status,
            top_line,
            0,
            0,
            0,
            0,
            Some(status_spans),
        );

        push(
            &mut lines,
            &mut line_old_num_left,
            &mut line_new_num_left,
            &mut line_old_num_right,
            &mut line_new_num_right,
            &mut file_header_status,
            build_split_border_blank(diff),
            0,
            0,
            0,
            0,
            None,
        );

        let meta_label = meta_file_label(&file.change);
        if !meta_label.is_empty() {
            push(
                &mut lines,
                &mut line_old_num_left,
                &mut line_new_num_left,
                &mut line_old_num_right,
                &mut line_new_num_right,
                &mut file_header_status,
                build_meta_hunk_header(diff, meta_label, 0, 0),
                0,
                0,
                0,
                0,
                None,
            );

            push(
                &mut lines,
                &mut line_old_num_left,
                &mut line_new_num_left,
                &mut line_old_num_right,
                &mut line_new_num_right,
                &mut file_header_status,
                build_split_border_blank(diff),
                0,
                0,
                0,
                0,
                None,
            );
        }

        match &file.change {
            FileChange::Binary => {
                let note_idx = lines.len();
                single_column_body_rows.push(note_idx);
                push(
                    &mut lines,
                    &mut line_old_num_left,
                    &mut line_new_num_left,
                    &mut line_old_num_right,
                    &mut line_new_num_right,
                    &mut file_header_status,
                    build_bordered_note(diff, " binary content not shown".to_string()),
                    0,
                    0,
                    0,
                    0,
                    None,
                );
            }
            FileChange::ModeOnly { old_mode, new_mode } => {
                let note_idx = lines.len();
                single_column_body_rows.push(note_idx);
                push(
                    &mut lines,
                    &mut line_old_num_left,
                    &mut line_new_num_left,
                    &mut line_old_num_right,
                    &mut line_new_num_right,
                    &mut file_header_status,
                    build_bordered_note(
                        diff,
                        format!(" {}", mode_change_label(old_mode, new_mode)),
                    ),
                    0,
                    0,
                    0,
                    0,
                    None,
                );
            }
            FileChange::Renamed { from, .. } => {
                let note_idx = lines.len();
                single_column_body_rows.push(note_idx);
                push(
                    &mut lines,
                    &mut line_old_num_left,
                    &mut line_new_num_left,
                    &mut line_old_num_right,
                    &mut line_new_num_right,
                    &mut file_header_status,
                    build_bordered_note(diff, format!(" renamed from {from}")),
                    0,
                    0,
                    0,
                    0,
                    None,
                );
            }
            _ => {}
        }

        if !meta_label.is_empty() && !file.hunks.is_empty() {
            push(
                &mut lines,
                &mut line_old_num_left,
                &mut line_new_num_left,
                &mut line_old_num_right,
                &mut line_new_num_right,
                &mut file_header_status,
                build_split_border_blank(diff),
                0,
                0,
                0,
                0,
                None,
            );
        }

        let hunk_count = file.hunks.len();
        for (hi, hunk) in file.hunks.iter().enumerate() {
            let use_syntect = hunk.lines.len() < SYNTECT_SKIP_HUNK_LINES;
            let (h_add, h_del) = count_hunk_changes(hunk);
            let header_line = build_hunk_header(hunk, diff, h_add, h_del, &file.change);
            push(
                &mut lines,
                &mut line_old_num_left,
                &mut line_new_num_left,
                &mut line_old_num_right,
                &mut line_new_num_right,
                &mut file_header_status,
                header_line,
                0,
                0,
                0,
                0,
                None,
            );
            push(
                &mut lines,
                &mut line_old_num_left,
                &mut line_new_num_left,
                &mut line_old_num_right,
                &mut line_new_num_right,
                &mut file_header_status,
                build_split_border_blank(diff),
                0,
                0,
                0,
                0,
                None,
            );

            let mut cur_old = hunk.header.old_start;
            let mut cur_new = hunk.header.new_start;

            let skip_leading = hunk
                .lines
                .iter()
                .take_while(|hl| matches!(hl, HunkLine::Context(s) if s.is_empty()))
                .count();
            let trailing_skip = hunk
                .lines
                .iter()
                .rev()
                .take(hunk.lines.len().saturating_sub(skip_leading))
                .take_while(|hl| matches!(hl, HunkLine::Context(s) if s.is_empty()))
                .count();
            for _ in 0..skip_leading {
                cur_old = cur_old.saturating_add(1);
                cur_new = cur_new.saturating_add(1);
            }
            let body_end_idx = hunk.lines.len() - trailing_skip;
            let slice = &hunk.lines[skip_leading..body_end_idx];
            let pairs = pair_hunk_lines(slice);
            for pair in pairs {
                let (line, ol, nl, or, nr) = match pair {
                    SplitPair::Context(s) => {
                        let o = cur_old;
                        let n = cur_new;
                        cur_old = cur_old.saturating_add(1);
                        cur_new = cur_new.saturating_add(1);
                        let l = build_split_body_line(
                            diff,
                            SplitSide::Context(s),
                            SplitSide::Context(s),
                            if use_syntect { hl_left.as_mut() } else { None },
                            if use_syntect { hl_right.as_mut() } else { None },
                            ss,
                        );
                        (l, o, n, o, n)
                    }
                    SplitPair::Change(d, a) => {
                        let o = cur_old;
                        let n = cur_new;
                        cur_old = cur_old.saturating_add(1);
                        cur_new = cur_new.saturating_add(1);
                        let l = build_split_body_line(
                            diff,
                            SplitSide::Del(d),
                            SplitSide::Add(a),
                            if use_syntect { hl_left.as_mut() } else { None },
                            if use_syntect { hl_right.as_mut() } else { None },
                            ss,
                        );
                        (l, o, 0, 0, n)
                    }
                    SplitPair::DelOnly(d) => {
                        let o = cur_old;
                        cur_old = cur_old.saturating_add(1);
                        let l = build_split_body_line(
                            diff,
                            SplitSide::Del(d),
                            SplitSide::Blank,
                            if use_syntect { hl_left.as_mut() } else { None },
                            if use_syntect { hl_right.as_mut() } else { None },
                            ss,
                        );
                        (l, o, 0, 0, 0)
                    }
                    SplitPair::AddOnly(a) => {
                        let n = cur_new;
                        cur_new = cur_new.saturating_add(1);
                        let l = build_split_body_line(
                            diff,
                            SplitSide::Blank,
                            SplitSide::Add(a),
                            if use_syntect { hl_left.as_mut() } else { None },
                            if use_syntect { hl_right.as_mut() } else { None },
                            ss,
                        );
                        (l, 0, 0, 0, n)
                    }
                    SplitPair::NoNewlineLeft => {
                        let l = build_split_body_line(
                            diff,
                            SplitSide::Context(NO_NEWLINE_MARKER),
                            SplitSide::Blank,
                            None,
                            None,
                            ss,
                        );
                        (l, 0, 0, 0, 0)
                    }
                    SplitPair::NoNewlineRight => {
                        let l = build_split_body_line(
                            diff,
                            SplitSide::Blank,
                            SplitSide::Context(NO_NEWLINE_MARKER),
                            None,
                            None,
                            ss,
                        );
                        (l, 0, 0, 0, 0)
                    }
                    SplitPair::NoNewlineBoth => {
                        let l = build_split_body_line(
                            diff,
                            SplitSide::Context(NO_NEWLINE_MARKER),
                            SplitSide::Context(NO_NEWLINE_MARKER),
                            None,
                            None,
                            ss,
                        );
                        (l, 0, 0, 0, 0)
                    }
                    SplitPair::Combined(prefix, content) => {
                        let bg = combined_row_bg(prefix, diff);
                        let mut all: Vec<Span<'static>> = Vec::new();
                        all.push(build_border_span_bare(diff));
                        all.extend(build_combined_half(prefix, content, bg, diff));
                        all.push(build_border_span_bare(diff));
                        all.extend(build_combined_half(prefix, content, bg, diff));
                        (Line::from(all), 0, 0, 0, 0)
                    }
                };
                push(
                    &mut lines,
                    &mut line_old_num_left,
                    &mut line_new_num_left,
                    &mut line_old_num_right,
                    &mut line_new_num_right,
                    &mut file_header_status,
                    line,
                    ol,
                    nl,
                    or,
                    nr,
                    None,
                );
            }

            if hi + 1 < hunk_count {
                push(
                    &mut lines,
                    &mut line_old_num_left,
                    &mut line_new_num_left,
                    &mut line_old_num_right,
                    &mut line_new_num_right,
                    &mut file_header_status,
                    build_split_border_blank(diff),
                    0,
                    0,
                    0,
                    0,
                    None,
                );
            }
        }

        let bottom_line_idx = lines.len();
        file_bottom_idx.push(bottom_line_idx);
        if file.hunks.is_empty() {
            meta_only_frame_rows.push(bottom_line_idx);
        }
        push(
            &mut lines,
            &mut line_old_num_left,
            &mut line_new_num_left,
            &mut line_old_num_right,
            &mut line_new_num_right,
            &mut file_header_status,
            build_file_bottom_frame(diff),
            0,
            0,
            0,
            0,
            None,
        );

        while file_owner.len() < top_line_idx {
            file_owner.push(None);
        }
        for _ in top_line_idx..=bottom_line_idx {
            file_owner.push(Some(fi));
        }

        if fi + 1 < files.len() {
            push(
                &mut lines,
                &mut line_old_num_left,
                &mut line_new_num_left,
                &mut line_old_num_right,
                &mut line_new_num_right,
                &mut file_header_status,
                Line::from(""),
                0,
                0,
                0,
                0,
                None,
            );
            file_owner.push(None);
        }
    }
    while file_owner.len() < lines.len() {
        file_owner.push(None);
    }

    let mut left_gutter_disabled: Vec<bool> = vec![false; lines.len()];
    let mut right_gutter_disabled: Vec<bool> = vec![false; lines.len()];
    for (fi, file) in files.iter().enumerate() {
        let top = file_top_idx[fi];
        let bottom = file_bottom_idx[fi];
        match &file.change {
            FileChange::Added => {
                for i in top..=bottom {
                    if i < left_gutter_disabled.len() {
                        left_gutter_disabled[i] = true;
                    }
                }
            }
            FileChange::Deleted => {
                for i in top..=bottom {
                    if i < right_gutter_disabled.len() {
                        right_gutter_disabled[i] = true;
                    }
                }
            }
            _ => {}
        }
    }

    assert_sidecars_parallel!(
        lines,
        line_old_num_left,
        line_new_num_left,
        line_old_num_right,
        line_new_num_right,
        file_header_status,
        file_owner,
        left_gutter_disabled,
        right_gutter_disabled,
    );

    BuiltSplitLines {
        lines,
        line_old_num_left,
        line_new_num_left,
        line_old_num_right,
        line_new_num_right,
        file_header_status,
        file_owner,
        file_top_idx,
        file_bottom_idx,
        meta_only_frame_rows,
        single_column_body_rows,
        left_gutter_disabled,
        right_gutter_disabled,
    }
}
