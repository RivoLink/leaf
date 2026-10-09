use super::*;
use crate::{
    diff::{is_mode_change_with_hunks, is_rename_with_hunks, DiffFile, FileChange, Hunk},
    theme::{app_theme, DiffTheme},
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub(super) fn build_file_top_frame_parts(
    file: &DiffFile,
    theme: &DiffTheme,
    added: usize,
    deleted: usize,
) -> (Line<'static>, Vec<Span<'static>>) {
    let path = if file.new_path.is_empty() || file.new_path == "/dev/null" {
        file.old_path.clone()
    } else {
        file.new_path.clone()
    };

    let path = if let FileChange::Renamed { to, .. } = &file.change {
        to.clone()
    } else {
        path
    };

    let status_label: String = match &file.change {
        FileChange::Modified => "modified".to_string(),
        FileChange::Added => "new file".to_string(),
        FileChange::Deleted => "deleted".to_string(),

        FileChange::Renamed { .. } if is_rename_with_hunks(file) => "modified".to_string(),

        FileChange::Renamed { .. } => "renamed".to_string(),
        FileChange::Binary => "binary".to_string(),

        FileChange::ModeOnly { .. } if is_mode_change_with_hunks(file) => "modified".to_string(),

        FileChange::ModeOnly { .. } => "mode changed".to_string(),
        FileChange::Submodule => "subproject".to_string(),
    };

    let show_counts = true;

    let app = app_theme();
    let frame_style = Style::default().fg(theme.frame_fg);
    let separator_style = Style::default()
        .fg(app.diff.file_separator_fg)
        .bg(app.ui.content_bg);
    let path_style = Style::default()
        .fg(theme.filename_fg)
        .add_modifier(Modifier::BOLD);

    let label_style = Style::default().fg(theme.hunk_header_fg);
    let add_style = Style::default().fg(theme.add_fg);
    let del_style = Style::default().fg(theme.del_fg);

    let line = Line::from(vec![
        Span::styled("┌─ ", frame_style),
        Span::styled(format!("{path} "), path_style),
    ]);
    let mut status_spans: Vec<Span<'static>> = Vec::with_capacity(6);
    status_spans.push(Span::styled(status_label, label_style));
    if show_counts {
        status_spans.push(Span::styled(" · ", separator_style));
        status_spans.push(Span::styled(format!("+{added}"), add_style));
        status_spans.push(Span::styled(" ", frame_style));
        status_spans.push(Span::styled(
            format!("{HUNK_HEADER_MINUS}{deleted}"),
            del_style,
        ));
    }
    (line, status_spans)
}

pub(super) fn replace_tee_at_col(line: &mut Line<'static>, col: usize) {
    let mut acc = 0usize;
    for span in line.spans.iter_mut() {
        let span_len = span.content.chars().count();
        if col < acc + span_len {
            let local = col - acc;
            let chars: Vec<char> = span.content.chars().collect();
            match chars.get(local).copied() {
                Some('┬') | Some('┴') | Some('┼') => {}
                _ => return,
            }
            let new_content: String = chars
                .into_iter()
                .enumerate()
                .map(|(i, c)| if i == local { '─' } else { c })
                .collect();
            span.content = new_content.into();
            return;
        }
        acc += span_len;
    }
}

pub(super) fn strip_mid_at_cols(line: &mut Line<'static>, cols: &[usize]) {
    for &col in cols {
        strip_mid_at_col(line, col);
    }
}

pub(super) fn strip_mid_at_col(line: &mut Line<'static>, col: usize) {
    let mut target: Option<usize> = None;
    let mut acc = 0usize;
    for (k, span) in line.spans.iter().enumerate() {
        let span_len = span.content.chars().count();
        if col < acc + span_len {
            target = Some(k);
            break;
        }
        acc += span_len;
    }
    let Some(target_idx) = target else { return };
    let local = col - acc;
    let chars: Vec<char> = line.spans[target_idx].content.chars().collect();
    match chars.get(local).copied() {
        Some('│') | Some('┬') | Some('┴') | Some('┼') => {}
        _ => return,
    }

    let left_char = if local > 0 {
        chars.get(local - 1).copied()
    } else {
        line.spans[..target_idx]
            .iter()
            .rev()
            .find_map(|s| s.content.chars().next_back())
    };
    let right_char = if local + 1 < chars.len() {
        chars.get(local + 1).copied()
    } else {
        line.spans[target_idx + 1..]
            .iter()
            .find_map(|s| s.content.chars().next())
    };
    let repl = if left_char == Some('─') || right_char == Some('─') {
        '─'
    } else {
        ' '
    };
    let new_content: String = chars
        .into_iter()
        .enumerate()
        .map(|(i, c)| if i == local { repl } else { c })
        .collect();
    line.spans[target_idx].content = new_content.into();
}

pub(super) fn build_meta_hunk_header(
    theme: &DiffTheme,
    label: &str,
    added: usize,
    deleted: usize,
) -> Line<'static> {
    let app = app_theme();
    let frame_style = Style::default().fg(theme.frame_fg);
    let separator_style = Style::default()
        .fg(app.diff.file_separator_fg)
        .bg(app.ui.content_bg);
    let anchor_style = Style::default().fg(theme.hunk_header_fg);
    let add_style = Style::default().fg(theme.add_fg);
    let del_style = Style::default().fg(theme.del_fg);
    Line::from(vec![
        Span::styled("├─ ", frame_style),
        Span::styled(label.to_string(), anchor_style),
        Span::styled(" · ", separator_style),
        Span::styled(format!("+{added}"), add_style),
        Span::styled(" ", frame_style),
        Span::styled(format!("{HUNK_HEADER_MINUS}{deleted}"), del_style),
    ])
}

pub(super) fn meta_file_label(change: &FileChange) -> &'static str {
    match change {
        FileChange::Binary => "binary",
        FileChange::ModeOnly { .. } => "mode changed",
        FileChange::Renamed { .. } => "renamed",
        _ => "",
    }
}

pub(super) fn dim_file_top_path(line: &mut Line<'static>, theme: &DiffTheme) {
    if let Some(span) = line.spans.get_mut(1) {
        span.style = Style::default().fg(theme.filename_fg);
    }
}

pub(super) fn recolor_frame_spans(line: &mut Line<'static>, from: Color, to: Color) {
    for span in line.spans.iter_mut() {
        if span.style.fg != Some(from) {
            continue;
        }
        let content: &str = &span.content;
        if content.is_empty() {
            continue;
        }

        let is_frame_only = content.chars().all(|c| {
            matches!(
                c,
                '┌' | '├'
                    | '└'
                    | '│'
                    | '┤'
                    | '┘'
                    | '┐'
                    | '─'
                    | '┬'
                    | '┴'
                    | '┼'
                    | '┊'
                    | ' '
            )
        });
        if is_frame_only {
            span.style.fg = Some(to);
        }
    }
}

pub(super) fn blend_rgb(a: Color, b: Color, ratio: f32) -> Color {
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let lerp = |x: u8, y: u8| -> u8 {
                (x as f32 * (1.0 - ratio) + y as f32 * ratio)
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            Color::Rgb(lerp(ar, br), lerp(ag, bg), lerp(ab, bb))
        }
        _ => a,
    }
}

pub(super) fn active_frame_tint(theme: &DiffTheme) -> Color {
    blend_rgb(theme.frame_fg, theme.filename_fg, 0.25)
}

pub(super) fn build_file_bottom_frame(theme: &DiffTheme) -> Line<'static> {
    Line::from(Span::styled("└─", Style::default().fg(theme.frame_fg)))
}

pub(super) fn build_border_span(theme: &DiffTheme) -> Span<'static> {
    Span::styled("│ ", Style::default().fg(theme.frame_fg))
}

pub(super) fn build_border_span_bare(theme: &DiffTheme) -> Span<'static> {
    Span::styled("│", Style::default().fg(theme.frame_fg))
}

pub(super) fn build_border_blank(theme: &DiffTheme) -> Line<'static> {
    Line::from(Span::styled("│", Style::default().fg(theme.frame_fg)))
}

pub(super) fn build_split_border_blank(theme: &DiffTheme) -> Line<'static> {
    let style = Style::default().fg(theme.frame_fg);
    Line::from(vec![Span::styled("│", style), Span::styled("│", style)])
}

pub(crate) fn mode_change_label(old_mode: &str, new_mode: &str) -> String {
    let is_regular = |m: &str| m.starts_with("100");
    let has_exec = |m: &str| {
        m.chars()
            .nth(3)
            .and_then(|c| c.to_digit(8))
            .is_some_and(|d| d & 0o1 != 0)
    };
    if is_regular(old_mode) && is_regular(new_mode) {
        match (has_exec(old_mode), has_exec(new_mode)) {
            (false, true) => return "executable permission added".to_string(),
            (true, false) => return "executable permission removed".to_string(),
            _ => {}
        }
    }
    let is_symlink = |m: &str| m.starts_with("120");
    match (is_symlink(old_mode), is_symlink(new_mode)) {
        (false, true) => return "converted to symlink".to_string(),
        (true, false) => return "converted from symlink".to_string(),
        _ => {}
    }
    "mode changed".to_string()
}

pub(super) fn build_bordered_note(theme: &DiffTheme, text: String) -> Line<'static> {
    let sigil_style = Style::default()
        .fg(theme.meta_note_sigil_fg)
        .bg(theme.meta_note_bg);
    let text_style = Style::default()
        .fg(theme.meta_note_fg)
        .bg(theme.meta_note_bg);
    Line::from(vec![
        Span::styled("│", Style::default().fg(theme.frame_fg)),
        Span::styled("!", sigil_style),
        Span::styled(text, text_style),
    ])
}

pub(super) fn build_hunk_header(
    hunk: &Hunk,
    theme: &DiffTheme,
    added: usize,
    deleted: usize,
    change: &FileChange,
) -> Line<'static> {
    let start = if hunk.header.new_start != 0 {
        hunk.header.new_start
    } else {
        hunk.header.old_start
    };
    let app = app_theme();
    let frame_style = Style::default().fg(theme.frame_fg);
    let separator_style = Style::default()
        .fg(app.diff.file_separator_fg)
        .bg(app.ui.content_bg);

    let anchor_fg = theme.hunk_header_fg;
    let around_style = Style::default().fg(anchor_fg);
    let line_num_style = Style::default().fg(anchor_fg);
    let add_style = Style::default().fg(theme.add_fg);
    let del_style = Style::default().fg(theme.del_fg);

    let mut spans: Vec<Span<'static>> = Vec::with_capacity(8);

    spans.push(Span::styled("├─ ", frame_style));

    match change {
        FileChange::Added => {
            spans.push(Span::styled("new file", around_style));
        }
        FileChange::Deleted => {
            spans.push(Span::styled("deleted", around_style));
        }
        FileChange::Submodule => {
            spans.push(Span::styled("subproject", around_style));
        }
        _ => {
            spans.push(Span::styled("around ", around_style));
            spans.push(Span::styled(format!("line {start}"), line_num_style));
        }
    }

    if added != 0 || deleted != 0 {
        spans.push(Span::styled(" · ", separator_style));
        spans.push(Span::styled(format!("+{added}"), add_style));
        spans.push(Span::styled(" ", frame_style));
        spans.push(Span::styled(
            format!("{HUNK_HEADER_MINUS}{deleted}"),
            del_style,
        ));
    }

    Line::from(spans)
}
