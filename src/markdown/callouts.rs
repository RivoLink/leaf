use super::spans::{normalize_html_tag, HtmlTagName};
use crate::theme::MarkdownTheme;
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CalloutKind {
    Note,
    Abstract,
    Info,
    Todo,
    Tip,
    Important,
    Success,
    Question,
    Warning,
    Caution,
    Failure,
    Danger,
    Bug,
    Example,
    Quote,
}

impl CalloutKind {
    const ALL: [Self; 15] = [
        Self::Note,
        Self::Abstract,
        Self::Info,
        Self::Todo,
        Self::Tip,
        Self::Important,
        Self::Success,
        Self::Question,
        Self::Warning,
        Self::Caution,
        Self::Failure,
        Self::Danger,
        Self::Bug,
        Self::Example,
        Self::Quote,
    ];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CalloutMarker {
    pub(crate) kind: CalloutKind,
    pub(crate) label: String,
    pub(crate) has_title: bool,
    pub(crate) marker_end: usize,
}

pub(crate) fn parse_callout_marker(src: &str, start: usize) -> Option<CalloutMarker> {
    let line_end = src[start..].find('\n').map_or(src.len(), |idx| start + idx);
    let bytes = src.as_bytes();
    let skip_blanks = |mut pos: usize| {
        while pos < line_end && matches!(bytes[pos], b' ' | b'\t') {
            pos += 1;
        }
        pos
    };

    let mut pos = skip_blanks(start);
    if bytes.get(pos) != Some(&b'>') {
        return None;
    }
    pos = skip_blanks(pos + 1);
    if !src[pos..line_end].starts_with("[!") {
        return None;
    }
    pos += 2;

    let name_start = pos;
    if !bytes.get(pos).is_some_and(u8::is_ascii_alphabetic) {
        return None;
    }
    while pos < line_end
        && (bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'_' | b'-'))
    {
        pos += 1;
    }
    let name = &src[name_start..pos];
    if bytes.get(pos) != Some(&b']') {
        return None;
    }
    pos += 1;
    if matches!(bytes.get(pos), Some(b'+' | b'-')) {
        pos += 1;
    }

    let marker_end = skip_blanks(pos);
    let has_title = !src[marker_end..line_end].trim().is_empty();
    Some(CalloutMarker {
        kind: callout_kind(name),
        label: default_label(name),
        has_title,
        marker_end,
    })
}

pub(crate) fn callout_kind(name: &str) -> CalloutKind {
    match name.to_ascii_lowercase().as_str() {
        "abstract" | "summary" | "tldr" => CalloutKind::Abstract,
        "info" => CalloutKind::Info,
        "todo" => CalloutKind::Todo,
        "tip" | "hint" => CalloutKind::Tip,
        "important" => CalloutKind::Important,
        "success" | "check" | "done" => CalloutKind::Success,
        "question" | "help" | "faq" => CalloutKind::Question,
        "warning" | "attention" => CalloutKind::Warning,
        "caution" => CalloutKind::Caution,
        "failure" | "fail" | "missing" => CalloutKind::Failure,
        "danger" | "error" => CalloutKind::Danger,
        "bug" => CalloutKind::Bug,
        "example" => CalloutKind::Example,
        "quote" | "cite" => CalloutKind::Quote,
        _ => CalloutKind::Note,
    }
}

pub(crate) fn default_label(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| {
        first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect()
    })
}

pub(super) fn callout_icon(kind: CalloutKind) -> &'static str {
    match kind {
        CalloutKind::Note | CalloutKind::Info => "[i]",
        CalloutKind::Abstract => "[=]",
        CalloutKind::Todo => "[o]",
        CalloutKind::Tip => "[*]",
        CalloutKind::Important | CalloutKind::Warning => "[!]",
        CalloutKind::Success => "[v]",
        CalloutKind::Question => "[?]",
        CalloutKind::Caution | CalloutKind::Failure | CalloutKind::Danger => "[x]",
        CalloutKind::Bug => "[#]",
        CalloutKind::Example => "[e]",
        CalloutKind::Quote => "[\"]",
    }
}

pub(super) fn callout_color(kind: CalloutKind, theme: &MarkdownTheme) -> Color {
    match kind {
        CalloutKind::Note | CalloutKind::Abstract | CalloutKind::Info | CalloutKind::Todo => {
            theme.alert_note
        }
        CalloutKind::Tip | CalloutKind::Success => theme.alert_tip,
        CalloutKind::Important | CalloutKind::Example => theme.alert_important,
        CalloutKind::Warning | CalloutKind::Question => theme.alert_warning,
        CalloutKind::Caution | CalloutKind::Failure | CalloutKind::Danger | CalloutKind::Bug => {
            theme.alert_caution
        }
        CalloutKind::Quote => theme.blockquote_marker,
    }
}

pub(super) fn callout_title_style(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

pub(crate) fn is_callout_icon(span: &Span<'_>, theme: &MarkdownTheme) -> bool {
    span.content.strip_suffix(' ').is_some_and(|icon| {
        CalloutKind::ALL.iter().any(|&kind| {
            icon == callout_icon(kind)
                && span.style == callout_title_style(callout_color(kind, theme))
        })
    })
}

pub(super) struct CalloutTitle {
    pub(super) marker: CalloutMarker,
    in_paragraph: bool,
}

pub(super) enum TitleStep {
    Pass,
    Skip,
    Trim(usize),
    Finish { consume: bool },
    Cancel,
}

impl CalloutTitle {
    pub(super) fn new(marker: CalloutMarker) -> Self {
        Self {
            marker,
            in_paragraph: false,
        }
    }

    pub(super) fn step(
        &mut self,
        ev: &pulldown_cmark::Event<'_>,
        range: &std::ops::Range<usize>,
    ) -> TitleStep {
        use pulldown_cmark::{Event, Tag, TagEnd};

        if !self.in_paragraph {
            if matches!(ev, Event::Start(Tag::Paragraph)) {
                if range.start > self.marker.marker_end {
                    return TitleStep::Cancel;
                }
                self.in_paragraph = true;
                return TitleStep::Skip;
            }
            return TitleStep::Finish { consume: false };
        }
        let marker_end = self.marker.marker_end;
        match ev {
            Event::SoftBreak | Event::HardBreak | Event::End(TagEnd::Paragraph) => {
                TitleStep::Finish { consume: true }
            }
            Event::InlineHtml(raw)
                if matches!(normalize_html_tag(raw), Some((HtmlTagName::Br, _))) =>
            {
                TitleStep::Finish { consume: true }
            }
            _ if range.end <= marker_end => TitleStep::Skip,
            Event::Text(text) if range.start < marker_end => {
                let offset = marker_end - range.start;
                if text.len() == range.len() && text.is_char_boundary(offset) {
                    TitleStep::Trim(offset)
                } else {
                    TitleStep::Pass
                }
            }
            _ => TitleStep::Pass,
        }
    }
}
