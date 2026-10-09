use super::*;
use crate::{diff::HunkLine, theme::DiffTheme};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use syntect::{easy::HighlightLines, highlighting::FontStyle, parsing::SyntaxSet};

pub(super) enum SplitSide<'a> {
    Context(&'a str),
    Del(&'a str),
    Add(&'a str),
    Blank,
}

pub(super) fn build_split_body_line(
    theme: &DiffTheme,
    left: SplitSide<'_>,
    right: SplitSide<'_>,
    hl_left: Option<&mut HighlightLines<'_>>,
    hl_right: Option<&mut HighlightLines<'_>>,
    ss: &SyntaxSet,
) -> Line<'static> {
    let border_style = Style::default().fg(theme.frame_fg);
    let mut spans: Vec<Span<'static>> = vec![Span::styled("│", border_style)];
    build_split_half_spans(&mut spans, theme, &left, hl_left, ss);
    spans.push(Span::styled("│", border_style));
    build_split_half_spans(&mut spans, theme, &right, hl_right, ss);
    Line::from(spans)
}

pub(super) fn build_split_half_spans(
    spans: &mut Vec<Span<'static>>,
    theme: &DiffTheme,
    side: &SplitSide<'_>,
    hl: Option<&mut HighlightLines<'_>>,
    ss: &SyntaxSet,
) {
    let (sigil_char, sigil_style, sep_style, body_bg, text_fg, text) = match side {
        SplitSide::Context(s) => (
            ' ',
            Style::default(),
            Style::default(),
            None,
            theme.context_fg,
            *s,
        ),
        SplitSide::Del(s) => (
            '-',
            Style::default().fg(theme.del_fg).bg(theme.del_bg),
            Style::default().bg(theme.del_bg),
            Some(theme.del_bg),
            theme.del_fg,
            *s,
        ),
        SplitSide::Add(s) => (
            '+',
            Style::default().fg(theme.add_fg).bg(theme.add_bg),
            Style::default().bg(theme.add_bg),
            Some(theme.add_bg),
            theme.add_fg,
            *s,
        ),
        SplitSide::Blank => (
            ' ',
            Style::default(),
            Style::default(),
            None,
            theme.context_fg,
            "",
        ),
    };

    spans.push(Span::styled(String::from(sigil_char), sigil_style));
    spans.push(Span::styled(" ", sep_style));
    let content_budget = SPLIT_COL_CONTENT_BUDGET;

    let mut content_spans: Vec<Span<'static>> = Vec::new();
    let mut char_used: usize = 0;
    if let Some(hl) = hl {
        let mut input = String::with_capacity(text.len() + 1);
        input.push_str(text);
        input.push('\n');
        if let Ok(regions) = hl.highlight_line(&input, ss) {
            for (st, chunk) in regions {
                let s = chunk.trim_end_matches('\n');
                if s.is_empty() {
                    continue;
                }
                let s_chars = s.chars().count();
                let remaining = content_budget.saturating_sub(char_used);
                if remaining == 0 {
                    break;
                }
                let take = s_chars.min(remaining);
                let trimmed: String = s.chars().take(take).collect();
                let mut style = Style::default().fg(syntect_to_color(st.foreground));
                if let Some(bg) = body_bg {
                    style = style.bg(bg);
                }
                if st.font_style.contains(FontStyle::BOLD) {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if st.font_style.contains(FontStyle::ITALIC) {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if st.font_style.contains(FontStyle::UNDERLINE) {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                content_spans.push(Span::styled(trimmed, style));
                char_used += take;
                if char_used >= content_budget {
                    break;
                }
            }
        }
    }
    if content_spans.is_empty() && !text.is_empty() {
        let take = text.chars().count().min(content_budget);
        let trimmed: String = text.chars().take(take).collect();
        let mut style = Style::default().fg(text_fg);
        if let Some(bg) = body_bg {
            style = style.bg(bg);
        }
        content_spans.push(Span::styled(trimmed, style));
    }

    if content_spans.is_empty() {
        let mut style = Style::default();
        if let Some(bg) = body_bg {
            style = style.bg(bg);
        }
        content_spans.push(Span::styled(String::new(), style));
    }
    spans.extend(content_spans);
}

pub(super) enum SplitPair<'a> {
    Context(&'a str),
    Change(&'a str, &'a str),
    DelOnly(&'a str),
    AddOnly(&'a str),
    NoNewlineLeft,
    NoNewlineRight,
    NoNewlineBoth,
    Combined(&'a str, &'a str),
}

pub(super) fn pair_hunk_lines(lines: &[HunkLine]) -> Vec<SplitPair<'_>> {
    let mut out: Vec<SplitPair<'_>> = Vec::new();
    let mut i = 0;

    let consume_markers = |i: &mut usize| -> bool {
        let start = *i;
        while *i < lines.len() && matches!(lines[*i], HunkLine::NoNewline) {
            *i += 1;
        }
        *i > start
    };
    while i < lines.len() {
        match &lines[i] {
            HunkLine::Context(s) => {
                out.push(SplitPair::Context(s));
                i += 1;
            }
            HunkLine::NoNewline => {
                let pair = match i.checked_sub(1).and_then(|j| lines.get(j)) {
                    Some(HunkLine::Add(_)) => SplitPair::NoNewlineRight,
                    _ => SplitPair::NoNewlineLeft,
                };
                out.push(pair);
                i += 1;
            }
            HunkLine::Del(_) => {
                let del_start = i;
                while i < lines.len() && matches!(lines[i], HunkLine::Del(_)) {
                    i += 1;
                }
                let del_end = i;
                let del_marker = consume_markers(&mut i);
                let add_start = i;
                while i < lines.len() && matches!(lines[i], HunkLine::Add(_)) {
                    i += 1;
                }
                let add_end = i;
                let add_marker = consume_markers(&mut i);
                let del_count = del_end - del_start;
                let add_count = add_end - add_start;
                let paired = del_count.min(add_count);
                for k in 0..paired {
                    let d = match &lines[del_start + k] {
                        HunkLine::Del(s) => s.as_str(),
                        _ => "",
                    };
                    let a = match &lines[add_start + k] {
                        HunkLine::Add(s) => s.as_str(),
                        _ => "",
                    };
                    out.push(SplitPair::Change(d, a));
                }
                for k in paired..del_count {
                    let d = match &lines[del_start + k] {
                        HunkLine::Del(s) => s.as_str(),
                        _ => "",
                    };
                    out.push(SplitPair::DelOnly(d));
                }
                for k in paired..add_count {
                    let a = match &lines[add_start + k] {
                        HunkLine::Add(s) => s.as_str(),
                        _ => "",
                    };
                    out.push(SplitPair::AddOnly(a));
                }
                let marker = match (del_marker, add_marker) {
                    (true, true) => Some(SplitPair::NoNewlineBoth),
                    (true, false) => Some(SplitPair::NoNewlineLeft),
                    (false, true) => Some(SplitPair::NoNewlineRight),
                    (false, false) => None,
                };
                if let Some(p) = marker {
                    out.push(p);
                }
            }
            HunkLine::Add(_) => {
                let add_start = i;
                while i < lines.len() && matches!(lines[i], HunkLine::Add(_)) {
                    i += 1;
                }
                for line in &lines[add_start..i] {
                    let a = match line {
                        HunkLine::Add(s) => s.as_str(),
                        _ => "",
                    };
                    out.push(SplitPair::AddOnly(a));
                }
                if consume_markers(&mut i) {
                    out.push(SplitPair::NoNewlineRight);
                }
            }

            HunkLine::Combined(prefix, content) => {
                out.push(SplitPair::Combined(prefix.as_str(), content.as_str()));
                i += 1;
            }
            HunkLine::WordDiff(_segments) => {
                out.push(SplitPair::Context(""));
                i += 1;
            }
        }
    }
    out
}
