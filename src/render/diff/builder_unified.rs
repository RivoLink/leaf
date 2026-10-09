use super::builder_common::{
    assert_sidecars_parallel, build_combined_half, combined_row_bg, SYNTECT_SKIP_HUNK_LINES,
};
use super::*;
use crate::{
    app::HunkRange,
    diff::{
        is_mode_change_with_hunks, is_rename_with_hunks, DiffFile, FileChange, HunkLine,
        NO_NEWLINE_MARKER,
    },
    theme::app_theme,
};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use syntect::{
    easy::HighlightLines,
    highlighting::Theme as SyntectTheme,
    parsing::{SyntaxReference, SyntaxSet},
};

pub(crate) struct BuiltDiffLines {
    pub(crate) lines: Vec<Line<'static>>,
    pub(crate) hunk_ranges: Vec<HunkRange>,
    pub(crate) line_owner: Vec<Option<(usize, usize)>>,
    pub(crate) line_old_num: Vec<u32>,
    pub(crate) line_new_num: Vec<u32>,
    pub(crate) file_header_status: Vec<Option<Vec<Span<'static>>>>,
    pub(crate) file_owner: Vec<Option<usize>>,
    pub(crate) file_top_idx: Vec<usize>,
    pub(crate) file_bottom_idx: Vec<usize>,
    pub(crate) meta_only_bottom_rows: Vec<usize>,
}

pub(crate) fn build_diff_lines(
    files: &[DiffFile],
    ss: &SyntaxSet,
    theme_syntect: &SyntectTheme,
) -> BuiltDiffLines {
    let theme = app_theme();
    let diff = &theme.diff;

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut hunk_ranges: Vec<HunkRange> = Vec::new();
    let mut line_owner: Vec<Option<(usize, usize)>> = Vec::new();
    let mut line_old_num: Vec<u32> = Vec::new();
    let mut line_new_num: Vec<u32> = Vec::new();
    let mut file_header_status: Vec<Option<Vec<Span<'static>>>> = Vec::new();
    let mut file_owner: Vec<Option<usize>> = Vec::new();
    let mut file_top_idx: Vec<usize> = Vec::new();
    let mut file_bottom_idx: Vec<usize> = Vec::new();
    let mut meta_only_bottom_rows: Vec<usize> = Vec::new();

    let push = |lines: &mut Vec<Line<'static>>,
                line_owner: &mut Vec<Option<(usize, usize)>>,
                line_old_num: &mut Vec<u32>,
                line_new_num: &mut Vec<u32>,
                file_header_status: &mut Vec<Option<Vec<Span<'static>>>>,
                line: Line<'static>,
                owner: Option<(usize, usize)>,
                old: u32,
                new: u32,
                status: Option<Vec<Span<'static>>>| {
        lines.push(line);
        line_owner.push(owner);
        line_old_num.push(old);
        line_new_num.push(new);
        file_header_status.push(status);
    };

    if files.is_empty() {
        push(
            &mut lines,
            &mut line_owner,
            &mut line_old_num,
            &mut line_new_num,
            &mut file_header_status,
            Line::from(Span::styled(
                "No changes to display".to_string(),
                Style::default().fg(diff.context_fg),
            )),
            None,
            0,
            0,
            None,
        );
        file_owner.push(None);
        return BuiltDiffLines {
            lines,
            hunk_ranges,
            line_owner,
            line_old_num,
            line_new_num,
            file_header_status,
            file_owner,
            file_top_idx,
            file_bottom_idx,
            meta_only_bottom_rows: Vec::new(),
        };
    }

    for (fi, file) in files.iter().enumerate() {
        let syntax: Option<&SyntaxReference> = resolve_file_syntax(file, ss);
        let mut hl = syntax.map(|s| HighlightLines::new(s, theme_syntect));

        let (file_add, file_del) = count_file_changes(file);
        let (top_line, status_spans) = build_file_top_frame_parts(file, diff, file_add, file_del);
        let top_line_idx = lines.len();
        file_top_idx.push(top_line_idx);
        push(
            &mut lines,
            &mut line_owner,
            &mut line_old_num,
            &mut line_new_num,
            &mut file_header_status,
            top_line,
            None,
            0,
            0,
            Some(status_spans),
        );
        push(
            &mut lines,
            &mut line_owner,
            &mut line_old_num,
            &mut line_new_num,
            &mut file_header_status,
            build_border_blank(diff),
            None,
            0,
            0,
            None,
        );

        let combined_meta_hunks = is_mode_change_with_hunks(file) || is_rename_with_hunks(file);
        let meta_label = meta_file_label(&file.change);
        if !meta_label.is_empty() {
            let (meta_add, meta_del) = if combined_meta_hunks {
                (0, 0)
            } else {
                (file_add, file_del)
            };
            push(
                &mut lines,
                &mut line_owner,
                &mut line_old_num,
                &mut line_new_num,
                &mut file_header_status,
                build_meta_hunk_header(diff, meta_label, meta_add, meta_del),
                None,
                0,
                0,
                None,
            );
            push(
                &mut lines,
                &mut line_owner,
                &mut line_old_num,
                &mut line_new_num,
                &mut file_header_status,
                build_border_blank(diff),
                None,
                0,
                0,
                None,
            );
        }
        match &file.change {
            FileChange::Binary => {
                push(
                    &mut lines,
                    &mut line_owner,
                    &mut line_old_num,
                    &mut line_new_num,
                    &mut file_header_status,
                    build_bordered_note(diff, " binary content not shown".to_string()),
                    None,
                    0,
                    0,
                    None,
                );
            }
            FileChange::ModeOnly { old_mode, new_mode } => {
                push(
                    &mut lines,
                    &mut line_owner,
                    &mut line_old_num,
                    &mut line_new_num,
                    &mut file_header_status,
                    build_bordered_note(
                        diff,
                        format!(" {}", mode_change_label(old_mode, new_mode)),
                    ),
                    None,
                    0,
                    0,
                    None,
                );
            }
            FileChange::Renamed { from, .. } => {
                push(
                    &mut lines,
                    &mut line_owner,
                    &mut line_old_num,
                    &mut line_new_num,
                    &mut file_header_status,
                    build_bordered_note(diff, format!(" renamed from {from}")),
                    None,
                    0,
                    0,
                    None,
                );
            }
            _ => {}
        }

        if combined_meta_hunks {
            push(
                &mut lines,
                &mut line_owner,
                &mut line_old_num,
                &mut line_new_num,
                &mut file_header_status,
                build_border_blank(diff),
                None,
                0,
                0,
                None,
            );
        }

        let hunk_count = file.hunks.len();
        for (hi, hunk) in file.hunks.iter().enumerate() {
            let use_syntect = hunk.lines.len() < SYNTECT_SKIP_HUNK_LINES;
            let (hunk_add, hunk_del) = count_hunk_changes(hunk);
            let header_line = build_hunk_header(hunk, diff, hunk_add, hunk_del, &file.change);
            let header_line_idx = lines.len();
            push(
                &mut lines,
                &mut line_owner,
                &mut line_old_num,
                &mut line_new_num,
                &mut file_header_status,
                header_line,
                Some((fi, hi)),
                0,
                0,
                None,
            );
            push(
                &mut lines,
                &mut line_owner,
                &mut line_old_num,
                &mut line_new_num,
                &mut file_header_status,
                build_border_blank(diff),
                Some((fi, hi)),
                0,
                0,
                None,
            );
            let body_start = lines.len();
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
            let intra_masks: Vec<IntraMaskPair> = compute_intra_masks(slice);
            for (li, hunk_line) in slice.iter().enumerate() {
                if matches!(hunk_line, HunkLine::NoNewline)
                    && slice[li + 1..]
                        .iter()
                        .any(|hl| matches!(hl, HunkLine::NoNewline))
                {
                    continue;
                }
                let (line, old_num, new_num) = match hunk_line {
                    HunkLine::Context(s) => {
                        let l = build_body_line(
                            &BodyLineStyle {
                                theme: diff,
                                fg: diff.context_fg,
                                bg: None,
                                intra_bg: None,
                            },
                            s,
                            if use_syntect { hl.as_mut() } else { None },
                            ss,
                            None,
                        );
                        let o = cur_old;
                        let n = cur_new;
                        cur_old = cur_old.saturating_add(1);
                        cur_new = cur_new.saturating_add(1);
                        (l, o, n)
                    }
                    HunkLine::Add(s) => {
                        let mask = intra_masks[li].1.as_ref();
                        let l = build_body_line(
                            &BodyLineStyle {
                                theme: diff,
                                fg: diff.add_fg,
                                bg: Some(diff.add_bg),
                                intra_bg: Some(diff.intra_add),
                            },
                            s,
                            if use_syntect { hl.as_mut() } else { None },
                            ss,
                            mask,
                        );
                        let n = cur_new;
                        cur_new = cur_new.saturating_add(1);
                        (l, 0, n)
                    }
                    HunkLine::Del(s) => {
                        let mask = intra_masks[li].0.as_ref();
                        let l = build_body_line(
                            &BodyLineStyle {
                                theme: diff,
                                fg: diff.del_fg,
                                bg: Some(diff.del_bg),
                                intra_bg: Some(diff.intra_del),
                            },
                            s,
                            if use_syntect { hl.as_mut() } else { None },
                            ss,
                            mask,
                        );
                        let o = cur_old;
                        cur_old = cur_old.saturating_add(1);
                        (l, o, 0)
                    }
                    HunkLine::NoNewline => {
                        let l = Line::from(vec![
                            build_border_span_bare(diff),
                            Span::raw(" "),
                            Span::raw(" "),
                            Span::styled(NO_NEWLINE_MARKER, Style::default().fg(diff.context_fg)),
                        ]);
                        (l, 0, 0)
                    }
                    HunkLine::Combined(prefix, content) => {
                        let bg = combined_row_bg(prefix, diff);
                        let mut spans: Vec<Span<'static>> =
                            Vec::with_capacity(prefix.chars().count() + 3);
                        spans.push(build_border_span_bare(diff));
                        spans.extend(build_combined_half(prefix, content, bg, diff));
                        let l = Line::from(spans);

                        cur_old = cur_old.saturating_add(1);
                        cur_new = cur_new.saturating_add(1);
                        (l, 0, 0)
                    }
                    HunkLine::WordDiff(segments) => {
                        let mut spans: Vec<Span<'static>> =
                            vec![build_border_span(diff), Span::raw(" ")];
                        for seg in segments {
                            let style = match seg.kind {
                                crate::diff::WordDiffKind::Common => {
                                    Style::default().fg(diff.context_fg)
                                }
                                crate::diff::WordDiffKind::Add => {
                                    Style::default().fg(diff.add_fg).bg(diff.add_bg)
                                }
                                crate::diff::WordDiffKind::Del => Style::default()
                                    .fg(diff.del_fg)
                                    .bg(diff.del_bg)
                                    .add_modifier(Modifier::CROSSED_OUT),
                            };
                            spans.push(Span::styled(seg.text.clone(), style));
                        }
                        let l = Line::from(spans);
                        let o = cur_old;
                        let n = cur_new;
                        cur_old = cur_old.saturating_add(1);
                        cur_new = cur_new.saturating_add(1);
                        (l, o, n)
                    }
                };
                push(
                    &mut lines,
                    &mut line_owner,
                    &mut line_old_num,
                    &mut line_new_num,
                    &mut file_header_status,
                    line,
                    Some((fi, hi)),
                    old_num,
                    new_num,
                    None,
                );
            }
            let body_end = lines.len();
            hunk_ranges.push(HunkRange {
                file_idx: fi,
                hunk_idx: hi,
                header_line: header_line_idx,
                body_start,
                body_end,
            });

            if hi + 1 < hunk_count {
                push(
                    &mut lines,
                    &mut line_owner,
                    &mut line_old_num,
                    &mut line_new_num,
                    &mut file_header_status,
                    build_border_blank(diff),
                    None,
                    0,
                    0,
                    None,
                );
            }
        }

        if let Some(nested) = &file.nested_submodule_diff {
            if !nested.is_empty() {
                push(
                    &mut lines,
                    &mut line_owner,
                    &mut line_old_num,
                    &mut line_new_num,
                    &mut file_header_status,
                    build_bordered_note(diff, "  submodule contents".to_string()),
                    None,
                    0,
                    0,
                    None,
                );
                for nf in nested {
                    let path = if nf.new_path.is_empty() || nf.new_path == "/dev/null" {
                        nf.old_path.clone()
                    } else {
                        nf.new_path.clone()
                    };
                    push(
                        &mut lines,
                        &mut line_owner,
                        &mut line_old_num,
                        &mut line_new_num,
                        &mut file_header_status,
                        build_bordered_note(diff, format!("    · {path}")),
                        None,
                        0,
                        0,
                        None,
                    );
                    if nf.nested_submodule_diff.is_some() {
                        push(
                            &mut lines,
                            &mut line_owner,
                            &mut line_old_num,
                            &mut line_new_num,
                            &mut file_header_status,
                            build_bordered_note(diff, "      (nested submodule)".to_string()),
                            None,
                            0,
                            0,
                            None,
                        );
                    }
                    for h in &nf.hunks {
                        push(
                            &mut lines,
                            &mut line_owner,
                            &mut line_old_num,
                            &mut line_new_num,
                            &mut file_header_status,
                            build_bordered_note(diff, format!("      {}", h.header_raw)),
                            None,
                            0,
                            0,
                            None,
                        );
                    }
                }
            }
        }

        let bottom_line_idx = lines.len();
        file_bottom_idx.push(bottom_line_idx);
        let meta_only_without_hunks = matches!(
            file.change,
            FileChange::ModeOnly { .. } | FileChange::Renamed { .. }
        ) && !combined_meta_hunks;
        if matches!(file.change, FileChange::Binary) || meta_only_without_hunks {
            meta_only_bottom_rows.push(bottom_line_idx);
        }
        push(
            &mut lines,
            &mut line_owner,
            &mut line_old_num,
            &mut line_new_num,
            &mut file_header_status,
            build_file_bottom_frame(diff),
            None,
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
                &mut line_owner,
                &mut line_old_num,
                &mut line_new_num,
                &mut file_header_status,
                Line::from(""),
                None,
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

    assert_sidecars_parallel!(
        lines,
        line_owner,
        line_old_num,
        line_new_num,
        file_header_status,
        file_owner,
    );

    BuiltDiffLines {
        lines,
        hunk_ranges,
        line_owner,
        line_old_num,
        line_new_num,
        file_header_status,
        file_owner,
        file_top_idx,
        file_bottom_idx,
        meta_only_bottom_rows,
    }
}

pub(super) fn count_file_changes(file: &DiffFile) -> (usize, usize) {
    let mut added = 0usize;
    let mut deleted = 0usize;
    for hunk in &file.hunks {
        let (a, d) = count_hunk_changes(hunk);
        added += a;
        deleted += d;
    }
    (added, deleted)
}

pub(super) fn count_hunk_changes(hunk: &crate::diff::Hunk) -> (usize, usize) {
    let mut added = 0usize;
    let mut deleted = 0usize;
    for line in &hunk.lines {
        match line {
            HunkLine::Add(_) => added += 1,
            HunkLine::Del(_) => deleted += 1,
            _ => {}
        }
    }
    (added, deleted)
}
