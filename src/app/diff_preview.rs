use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, Theme as SyntectTheme},
    parsing::SyntaxSet,
};

use crate::app::{DiffSource, DiffSpec, PreviewPair};
use crate::diff::{DiffFile, Hunk, HunkLine};
use crate::theme::app_theme;

fn is_usable_blob_sha(sha: &str) -> bool {
    !sha.is_empty() && !sha.chars().all(|c| c == '0')
}

fn git_stdout(spec: &DiffSpec, args: &[&str]) -> Option<String> {
    let out = spec.git_command().args(args).output().ok()?;
    out.status
        .success()
        .then(|| super::mode::decode_lossy(out.stdout))
}

fn git_show(spec: &DiffSpec, object: &str) -> Option<String> {
    git_stdout(spec, &["show", "--no-color", object])
}

fn blob(spec: &DiffSpec, sha: Option<&str>) -> Option<String> {
    sha.filter(|sha| is_usable_blob_sha(sha))
        .and_then(|sha| git_show(spec, sha))
}

fn fetch_before_blob(file: &DiffFile, spec: &DiffSpec) -> Option<String> {
    blob(spec, file.old_blob_sha.as_deref())
}

#[cfg(test)]
pub(crate) fn fetch_before(file: &DiffFile, spec: &DiffSpec) -> String {
    fetch_before_blob(file, spec).unwrap_or_else(|| fragment_sides(file).0)
}

pub(super) fn current_path(file: &DiffFile) -> &str {
    if file.new_path.is_empty() {
        &file.old_path
    } else {
        &file.new_path
    }
}

fn read_worktree_file(spec: &DiffSpec, path: &str) -> Option<String> {
    let root = git_stdout(spec, &["rev-parse", "--show-toplevel"])?;
    let bytes = std::fs::read(std::path::Path::new(root.trim()).join(path)).ok()?;
    Some(super::mode::decode_lossy(bytes))
}

fn fetch_after_source(file: &DiffFile, spec: &DiffSpec) -> Option<String> {
    let new_blob = || blob(spec, file.new_blob_sha.as_deref());
    match &spec.source {
        DiffSource::Ref(rev) if rev.contains("..") => new_blob(),
        DiffSource::Working | DiffSource::Ref(_) => read_worktree_file(spec, current_path(file)),
        DiffSource::Cached => git_show(spec, &format!(":{}", current_path(file))).or_else(new_blob),
        DiffSource::Revs(_, _) => new_blob(),
        DiffSource::Patch(_) => None,
    }
}

const PARTIAL_PREVIEW: &str = "source unavailable, showing hunks only";

fn full_sides(file: &DiffFile, spec: &DiffSpec) -> Option<(String, String)> {
    match &file.change {
        crate::diff::FileChange::Added => {
            let after = fetch_after_source(file, spec)
                .or_else(|| reconstruct_after_from_hunks("", file).ok())?;
            Some((String::new(), after))
        }
        crate::diff::FileChange::Deleted => Some((fetch_before_blob(file, spec)?, String::new())),
        _ if file.hunks.is_empty() => {
            let content = fetch_after_source(file, spec)
                .or_else(|| blob(spec, file.new_blob_sha.as_deref()))
                .or_else(|| fetch_before_blob(file, spec))
                .unwrap_or_default();
            Some((content.clone(), content))
        }
        _ => {
            let before = fetch_before_blob(file, spec)?;
            let after = match fetch_after_source(file, spec) {
                Some(after) => after,
                None => reconstruct_after_from_hunks(&before, file).ok()?,
            };
            Some((before, after))
        }
    }
}

fn fragment_sides(file: &DiffFile) -> (String, Vec<bool>, String, Vec<bool>) {
    let mut before: Vec<String> = Vec::new();
    let mut after: Vec<String> = Vec::new();
    let mut del_flags = Vec::new();
    let mut add_flags = Vec::new();
    for (hi, hunk) in file.hunks.iter().enumerate() {
        if hi > 0 {
            before.push("⋯".to_string());
            del_flags.push(false);
            after.push("⋯".to_string());
            add_flags.push(false);
        }
        for line in &hunk.lines {
            match line {
                HunkLine::Context(s) | HunkLine::Combined(_, s) => {
                    before.push(s.clone());
                    del_flags.push(false);
                    after.push(s.clone());
                    add_flags.push(false);
                }
                HunkLine::Del(s) => {
                    before.push(s.clone());
                    del_flags.push(true);
                }
                HunkLine::Add(s) => {
                    after.push(s.clone());
                    add_flags.push(true);
                }
                HunkLine::WordDiff(segments) => {
                    use crate::diff::{word_diff_side_text, WordDiffKind};
                    let (added, deleted) = crate::diff::word_diff_change_counts(segments);
                    before.push(word_diff_side_text(segments, WordDiffKind::Del));
                    del_flags.push(deleted > 0);
                    after.push(word_diff_side_text(segments, WordDiffKind::Add));
                    add_flags.push(added > 0);
                }
                HunkLine::NoNewline => {}
            }
        }
    }
    (before.join("\n"), del_flags, after.join("\n"), add_flags)
}

pub(crate) fn reconstruct_after_from_hunks(
    before: &str,
    file: &DiffFile,
) -> Result<String, String> {
    let before_lines: Vec<&str> = before.split('\n').collect();
    let mut out_lines: Vec<String> = Vec::new();
    let mut cursor: usize = 0;

    let mut hunks: Vec<&Hunk> = file.hunks.iter().collect();
    hunks.sort_by_key(|h| h.header.old_start);

    let expect = |cursor: usize, s: &str| -> Result<(), String> {
        match before_lines.get(cursor) {
            Some(line) if *line == s => Ok(()),
            _ => Err(format!(
                "hunk does not match the source at line {}",
                cursor + 1
            )),
        }
    };

    for hunk in &hunks {
        let hunk_old_start = if hunk.header.old_count == 0 {
            hunk.header.old_start as usize
        } else {
            hunk.header.old_start.saturating_sub(1) as usize
        };
        if hunk_old_start > before_lines.len() {
            return Err(format!(
                "hunk start {} past before EOF ({} lines)",
                hunk.header.old_start,
                before_lines.len()
            ));
        }
        while cursor < hunk_old_start && cursor < before_lines.len() {
            out_lines.push(before_lines[cursor].to_string());
            cursor += 1;
        }
        for hl in &hunk.lines {
            match hl {
                HunkLine::Context(s) => {
                    expect(cursor, s)?;
                    out_lines.push(s.clone());
                    cursor += 1;
                }
                HunkLine::Del(s) => {
                    expect(cursor, s)?;
                    cursor += 1;
                }
                HunkLine::Add(s) => {
                    out_lines.push(s.clone());
                }
                HunkLine::NoNewline => {}
                HunkLine::Combined(_prefix, content) => {
                    out_lines.push(content.clone());
                    cursor += 1;
                }
                HunkLine::WordDiff(segments) => {
                    let (uses_old, uses_new) = crate::diff::word_diff_sides(segments);
                    if uses_new {
                        out_lines.push(crate::diff::word_diff_side_text(
                            segments,
                            crate::diff::WordDiffKind::Add,
                        ));
                    }
                    if uses_old {
                        cursor += 1;
                    }
                }
            }
        }
    }
    while cursor < before_lines.len() {
        out_lines.push(before_lines[cursor].to_string());
        cursor += 1;
    }
    Ok(out_lines.join("\n"))
}

fn compute_diff_flags(file: &DiffFile) -> (Vec<bool>, Vec<bool>) {
    let mut del_flags: Vec<bool> = Vec::new();
    let mut add_flags: Vec<bool> = Vec::new();
    let mut hunks: Vec<&Hunk> = file.hunks.iter().collect();
    hunks.sort_by_key(|h| h.header.old_start);
    for hunk in &hunks {
        let mut cursor_old = hunk.header.old_start.saturating_sub(1) as usize;
        let mut cursor_new = hunk.header.new_start.saturating_sub(1) as usize;
        for hl in &hunk.lines {
            match hl {
                HunkLine::Context(_) => {
                    cursor_old += 1;
                    cursor_new += 1;
                }
                HunkLine::Del(_) => {
                    if del_flags.len() <= cursor_old {
                        del_flags.resize(cursor_old + 1, false);
                    }
                    del_flags[cursor_old] = true;
                    cursor_old += 1;
                }
                HunkLine::Add(_) => {
                    if add_flags.len() <= cursor_new {
                        add_flags.resize(cursor_new + 1, false);
                    }
                    add_flags[cursor_new] = true;
                    cursor_new += 1;
                }
                HunkLine::NoNewline => {}
                HunkLine::Combined(_, _) | HunkLine::WordDiff(_) => {}
            }
        }
    }
    (del_flags, add_flags)
}

use crate::render::{resolve_file_syntax, syntect_to_color};

fn render_side(
    raw: &str,
    flags: &[bool],
    is_del_side: bool,
    ss: &SyntaxSet,
    theme_syntect: &SyntectTheme,
    syntax: Option<&syntect::parsing::SyntaxReference>,
) -> Vec<Line<'static>> {
    let theme = &app_theme().diff;

    let active_tint = {
        match (theme.frame_fg, theme.filename_fg) {
            (ratatui::style::Color::Rgb(ar, ag, ab), ratatui::style::Color::Rgb(br, bg, bb)) => {
                let lerp = |x: u8, y: u8| -> u8 {
                    (x as f32 * 0.75 + y as f32 * 0.25)
                        .round()
                        .clamp(0.0, 255.0) as u8
                };
                ratatui::style::Color::Rgb(lerp(ar, br), lerp(ag, bg), lerp(ab, bb))
            }
            _ => theme.filename_fg,
        }
    };
    let frame_style = Style::default().fg(active_tint);
    let plain_sigil_fg = theme.frame_fg;
    let side_bg = if is_del_side {
        theme.del_bg
    } else {
        theme.add_bg
    };
    let side_sigil_fg = if is_del_side {
        theme.del_fg
    } else {
        theme.add_fg
    };
    let sigil_char = if is_del_side { '-' } else { '+' };

    let body = raw.strip_suffix('\n').unwrap_or(raw);
    let lines_src: Vec<&str> = body.split('\n').collect();
    let total = lines_src.len();
    let digit_w = total.max(1).to_string().len();

    let plain_syntax = ss.find_syntax_plain_text();
    let syntax_ref = syntax.unwrap_or(plain_syntax);
    let mut hl = HighlightLines::new(syntax_ref, theme_syntect);

    let mut out: Vec<Line<'static>> = Vec::with_capacity(total);
    for (i, src_line) in lines_src.into_iter().enumerate() {
        let hit = flags.get(i).copied().unwrap_or(false);
        let (sigil, row_bg): (char, Option<ratatui::style::Color>) = if hit {
            (sigil_char, Some(side_bg))
        } else {
            (' ', None)
        };

        let (mut line_for_syntect, _) = crate::render::display_text(src_line, 0);
        line_for_syntect.push('\n');
        let display_line = &line_for_syntect[..line_for_syntect.len() - 1];

        let ranges = hl
            .highlight_line(&line_for_syntect, ss)
            .unwrap_or_else(|_| vec![(syntect::highlighting::Style::default(), display_line)]);

        let mut spans: Vec<Span<'static>> = Vec::with_capacity(ranges.len() + 4);

        let line_num = i + 1;
        let gutter_text = format!("{line_num:>w$}", w = digit_w);
        let num_fg = if hit { side_sigil_fg } else { theme.frame_fg };
        let mut num_style = Style::default().fg(num_fg);
        if let Some(bg) = row_bg {
            num_style = num_style.bg(bg);
        }
        spans.push(Span::styled(gutter_text, num_style));

        let div_style = if let Some(bg) = row_bg {
            frame_style.bg(bg)
        } else {
            frame_style
        };
        spans.push(Span::styled("│", div_style));

        let sigil_fg = if hit { side_sigil_fg } else { plain_sigil_fg };
        let mut sigil_style = Style::default().fg(sigil_fg);
        if let Some(bg) = row_bg {
            sigil_style = sigil_style.bg(bg);
        }
        spans.push(Span::styled(format!("{sigil} "), sigil_style));

        for (sty, text) in ranges {
            let text = text.trim_end_matches('\n');
            if text.is_empty() {
                continue;
            }
            let mut span_style = Style::default().fg(syntect_to_color(sty.foreground));
            let fs = sty.font_style;
            if fs.contains(FontStyle::BOLD) {
                span_style = span_style.add_modifier(Modifier::BOLD);
            }
            if fs.contains(FontStyle::ITALIC) {
                span_style = span_style.add_modifier(Modifier::ITALIC);
            }
            if fs.contains(FontStyle::UNDERLINE) {
                span_style = span_style.add_modifier(Modifier::UNDERLINED);
            }
            if let Some(bg) = row_bg {
                span_style = span_style.bg(bg);
            }
            spans.push(Span::styled(text.to_string(), span_style));
        }

        out.push(Line::from(spans));
    }
    out
}

pub(crate) fn build_preview_pair(
    file: &DiffFile,
    spec: &DiffSpec,
    ss: &SyntaxSet,
    theme_syntect: &SyntectTheme,
) -> PreviewPair {
    if file.is_binary() {
        let note = preview_empty_side_note(" binary content not shown");
        return PreviewPair {
            before_lines: note.clone(),
            after_lines: note,
            before_is_note: true,
            after_is_note: true,
            after_error: None,
            first_change_row: 0,
        };
    }

    let syntax = resolve_file_syntax(file, ss);
    let (full, after_error) = match file.change {
        crate::diff::FileChange::Submodule => (None, None),
        _ => {
            let full = full_sides(file, spec);
            let after_error = full.is_none().then(|| PARTIAL_PREVIEW.to_string());
            (full, after_error)
        }
    };
    let (before_raw, del_flags, after_raw, add_flags) = match full {
        Some((before, after)) => {
            let (del_flags, add_flags) = compute_diff_flags(file);
            (before, del_flags, after, add_flags)
        }
        None => fragment_sides(file),
    };
    let first_change_row = del_flags
        .iter()
        .position(|&hit| hit)
        .into_iter()
        .chain(add_flags.iter().position(|&hit| hit))
        .min()
        .unwrap_or(0);
    let render = |raw: &str, flags: &[bool], is_del: bool| {
        render_side(raw, flags, is_del, ss, theme_syntect, syntax)
    };

    let meta_only_note = match &file.change {
        crate::diff::FileChange::ModeOnly { old_mode, new_mode } if file.hunks.is_empty() => Some(
            format!(" {}", crate::render::mode_change_label(old_mode, new_mode)),
        ),
        crate::diff::FileChange::Renamed { from, .. } if file.hunks.is_empty() => {
            Some(format!(" renamed from {from}"))
        }
        _ => None,
    };
    let (before_lines, after_lines, before_is_note, after_is_note) = match &file.change {
        crate::diff::FileChange::Added => (
            preview_empty_side_note(" new file"),
            render(&after_raw, &add_flags, false),
            true,
            false,
        ),
        crate::diff::FileChange::Deleted => (
            render(&before_raw, &del_flags, true),
            preview_empty_side_note(" file deleted"),
            false,
            true,
        ),
        _ if meta_only_note.is_some() => (
            render(&after_raw, &[], false),
            preview_empty_side_note(meta_only_note.as_deref().unwrap_or_default()),
            false,
            true,
        ),
        _ => (
            render(&before_raw, &del_flags, true),
            render(&after_raw, &add_flags, false),
            false,
            false,
        ),
    };
    PreviewPair {
        before_lines,
        after_lines,
        before_is_note,
        after_is_note,
        after_error,
        first_change_row,
    }
}

fn preview_empty_side_note(text: &str) -> Vec<Line<'static>> {
    let theme = &app_theme().diff;
    let sigil_style = Style::default()
        .fg(theme.meta_note_sigil_fg)
        .bg(theme.meta_note_bg);
    let text_style = Style::default()
        .fg(theme.meta_note_fg)
        .bg(theme.meta_note_bg);
    vec![Line::from(vec![
        Span::styled("!", sigil_style),
        Span::styled(text.to_string(), text_style),
    ])]
}
