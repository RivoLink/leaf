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

pub(crate) fn fetch_before(file: &DiffFile, _spec: &DiffSpec) -> String {
    if let Some(sha) = file.old_blob_sha.as_deref() {
        if is_usable_blob_sha(sha) {
            if let Ok(out) = std::process::Command::new("git")
                .arg("show")
                .arg(sha)
                .output()
            {
                if out.status.success() {
                    return String::from_utf8_lossy(&out.stdout).into_owned();
                }
            }
        }
    }
    reconstruct_before_from_hunks(file)
}

pub(crate) fn fetch_after(file: &DiffFile, spec: &DiffSpec) -> (String, Option<String>) {
    match &spec.source {
        DiffSource::Working => {
            let path = if !file.new_path.is_empty() && file.new_path != "/dev/null" {
                file.new_path.as_str()
            } else {
                file.old_path.as_str()
            };
            if let Ok(s) = std::fs::read_to_string(path) {
                return (s, None);
            }
        }
        DiffSource::Cached => {
            let path = if !file.new_path.is_empty() && file.new_path != "/dev/null" {
                file.new_path.as_str()
            } else {
                file.old_path.as_str()
            };
            if let Ok(out) = std::process::Command::new("git")
                .arg("show")
                .arg(format!(":{path}"))
                .output()
            {
                if out.status.success() {
                    return (String::from_utf8_lossy(&out.stdout).into_owned(), None);
                }
            }
        }
        DiffSource::Ref(_) => {}
    }
    if let Some(sha) = file.new_blob_sha.as_deref() {
        if is_usable_blob_sha(sha) {
            if let Ok(out) = std::process::Command::new("git")
                .arg("show")
                .arg(sha)
                .output()
            {
                if out.status.success() {
                    return (String::from_utf8_lossy(&out.stdout).into_owned(), None);
                }
            }
        }
    }
    let before = fetch_before(file, spec);
    match reconstruct_after_from_hunks(&before, file) {
        Ok(s) => (s, None),
        Err(e) => (before, Some(e)),
    }
}

fn reconstruct_before_from_hunks(file: &DiffFile) -> String {
    let mut out = String::new();
    for hunk in &file.hunks {
        for l in &hunk.lines {
            match l {
                HunkLine::Context(s) | HunkLine::Del(s) => {
                    out.push_str(s);
                    out.push('\n');
                }

                HunkLine::Combined(_, s) => {
                    out.push_str(s);
                    out.push('\n');
                }
                _ => {}
            }
        }
    }
    out
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

    for hunk in &hunks {
        let hunk_old_start = hunk.header.old_start.saturating_sub(1) as usize;
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
                    out_lines.push(s.clone());
                    cursor += 1;
                }
                HunkLine::Del(_) => {
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
                    let mut rebuilt = String::new();
                    for seg in segments {
                        match seg.kind {
                            crate::diff::WordDiffKind::Common | crate::diff::WordDiffKind::Add => {
                                rebuilt.push_str(&seg.text);
                            }
                            crate::diff::WordDiffKind::Del => {}
                        }
                    }
                    out_lines.push(rebuilt);
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

        let line_for_syntect = format!("{src_line}\n");

        let ranges = hl
            .highlight_line(&line_for_syntect, ss)
            .unwrap_or_else(|_| vec![(syntect::highlighting::Style::default(), src_line)]);

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
    let before_raw = fetch_before(file, spec);
    let (after_raw, after_error) = fetch_after(file, spec);
    let (del_flags, add_flags) = compute_diff_flags(file);
    let syntax = resolve_file_syntax(file, ss);
    let before_lines = render_side(&before_raw, &del_flags, true, ss, theme_syntect, syntax);
    let after_lines = render_side(&after_raw, &add_flags, false, ss, theme_syntect, syntax);

    let (before_lines, after_lines, before_is_note, after_is_note) = match &file.change {
        crate::diff::FileChange::Added => (
            preview_empty_side_note(" new file"),
            after_lines,
            true,
            false,
        ),
        crate::diff::FileChange::Deleted => (
            before_lines,
            preview_empty_side_note(" file deleted"),
            false,
            true,
        ),
        crate::diff::FileChange::ModeOnly { old_mode, new_mode } => {
            if !file.hunks.is_empty() {
                (before_lines, after_lines, false, false)
            } else {
                let text = format!(" {}", crate::render::mode_change_label(old_mode, new_mode));
                let before_lines = if before_raw.is_empty() {
                    after_lines.clone()
                } else {
                    before_lines
                };
                (before_lines, preview_empty_side_note(&text), false, true)
            }
        }
        crate::diff::FileChange::Renamed { from, .. } => {
            if !file.hunks.is_empty() {
                (before_lines, after_lines, false, false)
            } else {
                let text = format!(" renamed from {from}");
                let before_lines = if before_raw.is_empty() {
                    after_lines.clone()
                } else {
                    before_lines
                };
                (before_lines, preview_empty_side_note(&text), false, true)
            }
        }
        crate::diff::FileChange::Binary => {
            let note = preview_empty_side_note(" binary content not shown");
            (note.clone(), note, true, true)
        }
        crate::diff::FileChange::Submodule => (before_lines, after_lines, false, false),
        _ => (before_lines, after_lines, false, false),
    };
    PreviewPair {
        before_lines,
        after_lines,
        before_is_note,
        after_is_note,
        after_error,
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
