use crate::theme::MarkdownTheme;
use pulldown_cmark::{Event as MdEvent, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use super::latex;
use super::links::{register_link, LinkId, LinkedSpan};
use super::with_link_marker;

#[derive(Clone, Copy, Default)]
pub(super) struct InlineStyleState {
    pub(super) in_strong: u8,
    pub(super) in_em: u8,
    pub(super) in_strike: u8,
    pub(super) in_underline: u8,
    pub(super) link_id: Option<LinkId>,
}

impl InlineStyleState {
    pub(super) fn modifiers(&self) -> Modifier {
        let mut m = Modifier::empty();
        if self.in_strong > 0 {
            m |= Modifier::BOLD;
        }
        if self.in_em > 0 {
            m |= Modifier::ITALIC;
        }
        if self.in_strike > 0 {
            m |= Modifier::CROSSED_OUT;
        }
        if self.in_underline > 0 {
            m |= Modifier::UNDERLINED;
        }
        m
    }

    pub(super) fn reset_html_counters(&mut self) {
        self.in_strong = 0;
        self.in_em = 0;
        self.in_strike = 0;
        self.in_underline = 0;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HtmlBufferKind {
    Mark,
    Code,
}

pub(super) enum HtmlTagOutcome {
    Consumed,
    OpenStyleBuffer(HtmlBufferKind),
    CloseStyleBuffer(HtmlBufferKind),
    HardBreak,
    NotRecognized,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HtmlTagName {
    Bold,
    Italic,
    Strike,
    Underline,
    Mark,
    Code,
    Br,
}

fn classify_tag_name(name: &str) -> Option<HtmlTagName> {
    if name.eq_ignore_ascii_case("b") || name.eq_ignore_ascii_case("strong") {
        Some(HtmlTagName::Bold)
    } else if name.eq_ignore_ascii_case("i") || name.eq_ignore_ascii_case("em") {
        Some(HtmlTagName::Italic)
    } else if name.eq_ignore_ascii_case("s") || name.eq_ignore_ascii_case("del") {
        Some(HtmlTagName::Strike)
    } else if name.eq_ignore_ascii_case("u") || name.eq_ignore_ascii_case("ins") {
        Some(HtmlTagName::Underline)
    } else if name.eq_ignore_ascii_case("mark") {
        Some(HtmlTagName::Mark)
    } else if name.eq_ignore_ascii_case("code") {
        Some(HtmlTagName::Code)
    } else if name.eq_ignore_ascii_case("br") {
        Some(HtmlTagName::Br)
    } else {
        None
    }
}

pub(super) fn normalize_html_tag(raw: &str) -> Option<(HtmlTagName, bool)> {
    let s = raw.trim_start();
    if !s.starts_with('<') {
        return None;
    }
    let s = s.trim_end();
    let s = s.strip_prefix('<')?.strip_suffix('>')?;
    let (s, is_open) = match s.strip_prefix('/') {
        Some(rest) => (rest, false),
        None => (s, true),
    };
    let s = s.trim_end().trim_end_matches('/').trim_end();
    let end = s
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(s.len());
    let name = &s[..end];
    if name.is_empty() {
        return None;
    }
    Some((classify_tag_name(name)?, is_open))
}

pub(super) fn handle_html_tag_event(
    raw: &str,
    inline: &mut InlineStyleState,
    spans: &mut [LinkedSpan],
) -> HtmlTagOutcome {
    let Some((tag, is_open)) = normalize_html_tag(raw) else {
        return HtmlTagOutcome::NotRecognized;
    };
    match (tag, is_open) {
        (HtmlTagName::Bold, true) => {
            inline.in_strong = inline.in_strong.saturating_add(1);
            if inline.link_id.is_some() {
                update_link_marker_modifier(spans, Modifier::BOLD);
            }
        }
        (HtmlTagName::Bold, false) => {
            inline.in_strong = inline.in_strong.saturating_sub(1);
        }
        (HtmlTagName::Italic, true) => {
            inline.in_em = inline.in_em.saturating_add(1);
            if inline.link_id.is_some() {
                update_link_marker_modifier(spans, Modifier::ITALIC);
            }
        }
        (HtmlTagName::Italic, false) => {
            inline.in_em = inline.in_em.saturating_sub(1);
        }
        (HtmlTagName::Strike, true) => {
            inline.in_strike = inline.in_strike.saturating_add(1);
            if inline.link_id.is_some() {
                update_link_marker_modifier(spans, Modifier::CROSSED_OUT);
            }
        }
        (HtmlTagName::Strike, false) => {
            inline.in_strike = inline.in_strike.saturating_sub(1);
        }
        (HtmlTagName::Underline, true) => {
            inline.in_underline = inline.in_underline.saturating_add(1);
            if inline.link_id.is_some() {
                update_link_marker_modifier(spans, Modifier::UNDERLINED);
            }
        }
        (HtmlTagName::Underline, false) => {
            inline.in_underline = inline.in_underline.saturating_sub(1);
        }
        (HtmlTagName::Mark, true) => return HtmlTagOutcome::OpenStyleBuffer(HtmlBufferKind::Mark),
        (HtmlTagName::Mark, false) => {
            return HtmlTagOutcome::CloseStyleBuffer(HtmlBufferKind::Mark)
        }
        (HtmlTagName::Code, true) => return HtmlTagOutcome::OpenStyleBuffer(HtmlBufferKind::Code),
        (HtmlTagName::Code, false) => {
            return HtmlTagOutcome::CloseStyleBuffer(HtmlBufferKind::Code)
        }
        (HtmlTagName::Br, _) => return HtmlTagOutcome::HardBreak,
    }
    HtmlTagOutcome::Consumed
}

pub(super) fn inline_text_style(
    theme: &MarkdownTheme,
    blockquote_depth: usize,
    inline: InlineStyleState,
) -> Style {
    let mut style = if inline.link_id.is_some() {
        let mut s = Style::default()
            .fg(theme.link_text)
            .add_modifier(Modifier::UNDERLINED);
        if blockquote_depth > 0 {
            s = s.add_modifier(Modifier::ITALIC);
        }
        s
    } else if blockquote_depth > 0 {
        Style::default()
            .fg(theme.blockquote_text)
            .add_modifier(Modifier::ITALIC)
    } else {
        Style::default().fg(theme.text)
    };

    if inline.in_strong > 0 && inline.link_id.is_none() {
        style = style.fg(theme.strong_text);
    }
    style = style.add_modifier(inline.modifiers());

    style
}

pub(super) fn handle_inline_style_event(
    ev: &MdEvent<'_>,
    inline: &mut InlineStyleState,
    spans: &mut Vec<LinkedSpan>,
    theme: &MarkdownTheme,
    blockquote_depth: usize,
    link_urls: &mut Vec<String>,
) -> bool {
    match ev {
        MdEvent::Start(Tag::Strong) => {
            inline.in_strong = inline.in_strong.saturating_add(1);
            if inline.link_id.is_some() {
                update_link_marker_modifier(spans, Modifier::BOLD);
            }
            true
        }
        MdEvent::End(TagEnd::Strong) => {
            inline.in_strong = inline.in_strong.saturating_sub(1);
            true
        }
        MdEvent::Start(Tag::Emphasis) => {
            inline.in_em = inline.in_em.saturating_add(1);
            if inline.link_id.is_some() {
                update_link_marker_modifier(spans, Modifier::ITALIC);
            }
            true
        }
        MdEvent::End(TagEnd::Emphasis) => {
            inline.in_em = inline.in_em.saturating_sub(1);
            true
        }
        MdEvent::Start(Tag::Strikethrough) => {
            inline.in_strike = inline.in_strike.saturating_add(1);
            if inline.link_id.is_some() {
                update_link_marker_modifier(spans, Modifier::CROSSED_OUT);
            }
            true
        }
        MdEvent::End(TagEnd::Strikethrough) => {
            inline.in_strike = inline.in_strike.saturating_sub(1);
            true
        }
        MdEvent::Start(Tag::Link { dest_url, .. }) => {
            inline.link_id = Some(register_link(link_urls, dest_url.as_ref()));
            push_link_marker(spans, theme, *inline, blockquote_depth);
            true
        }
        MdEvent::End(TagEnd::Link) => {
            inline.link_id = None;
            true
        }
        _ => false,
    }
}

pub(super) fn push_inline_code_span(
    spans: &mut Vec<LinkedSpan>,
    text: &str,
    theme: &MarkdownTheme,
    link_id: Option<LinkId>,
) {
    spans.push(LinkedSpan::new(
        Span::styled(
            format!(" {} ", text),
            Style::default()
                .fg(theme.inline_code_fg)
                .bg(theme.inline_code_bg),
        ),
        link_id,
    ));
}

pub(super) fn push_mark_span(
    spans: &mut Vec<LinkedSpan>,
    text: &str,
    theme: &MarkdownTheme,
    link_id: Option<LinkId>,
) {
    spans.push(LinkedSpan::new(
        Span::styled(
            format!(" {} ", text),
            Style::default().fg(theme.mark_fg).bg(theme.mark_bg),
        ),
        link_id,
    ));
}

pub(super) fn push_inline_latex_span(
    spans: &mut Vec<LinkedSpan>,
    text: &str,
    theme: &MarkdownTheme,
    link_id: Option<LinkId>,
) {
    let rendered = latex::to_unicode(text);
    spans.push(LinkedSpan::new(
        Span::styled(
            format!(" {rendered} "),
            Style::default()
                .fg(theme.latex_inline_fg)
                .bg(theme.latex_inline_bg),
        ),
        link_id,
    ));
}

pub(super) fn push_link_marker(
    spans: &mut Vec<LinkedSpan>,
    theme: &MarkdownTheme,
    inline: InlineStyleState,
    blockquote_depth: usize,
) {
    let mut style = Style::default()
        .fg(theme.link_icon)
        .add_modifier(inline.modifiers());
    if blockquote_depth > 0 {
        style = style.add_modifier(Modifier::ITALIC);
    }
    with_link_marker(|m| {
        spans.push(LinkedSpan::new(
            Span::styled(m.to_string(), style),
            inline.link_id,
        ))
    });
}

pub(super) fn update_link_marker_modifier(spans: &mut [LinkedSpan], modifier: Modifier) {
    with_link_marker(|marker| {
        if let Some(LinkedSpan { span, .. }) = spans
            .iter_mut()
            .rev()
            .find(|s| s.span.content.as_ref() == marker)
        {
            span.style = span.style.add_modifier(modifier);
        }
    });
}
