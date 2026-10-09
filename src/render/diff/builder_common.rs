pub(super) const SYNTECT_SKIP_HUNK_LINES: usize = 2000;

macro_rules! assert_sidecars_parallel {
    ($lines:expr, $($sidecar:expr),+ $(,)?) => {
        $(
            debug_assert_eq!(
                $lines.len(),
                $sidecar.len(),
                concat!("sidecar ", stringify!($sidecar), " out of sync with lines"),
            );
        )+
    };
}

pub(super) use assert_sidecars_parallel;

pub(super) fn combined_row_bg(
    prefix: &str,
    diff: &crate::theme::DiffTheme,
) -> Option<ratatui::style::Color> {
    let has_plus = prefix.chars().any(|c| c == '+');
    let has_minus = prefix.chars().any(|c| c == '-');
    match (has_plus, has_minus) {
        (true, true) => Some(diff.meta_note_bg),
        (true, false) => Some(diff.add_bg),
        (false, true) => Some(diff.del_bg),
        (false, false) => None,
    }
}

fn combined_prefix_spans(
    prefix: &str,
    diff: &crate::theme::DiffTheme,
    bg: Option<ratatui::style::Color>,
) -> Vec<ratatui::text::Span<'static>> {
    use ratatui::style::Style;
    use ratatui::text::Span;
    prefix
        .chars()
        .map(|ch| {
            let fg = match ch {
                '+' => diff.add_fg,
                '-' => diff.del_fg,
                _ => diff.frame_fg,
            };
            let mut style = Style::default().fg(fg);
            if let Some(bg) = bg {
                style = style.bg(bg);
            }
            Span::styled(ch.to_string(), style)
        })
        .collect()
}

pub(super) fn build_combined_half(
    prefix: &str,
    content: &str,
    bg: Option<ratatui::style::Color>,
    diff: &crate::theme::DiffTheme,
) -> Vec<ratatui::text::Span<'static>> {
    use ratatui::style::Style;
    use ratatui::text::Span;
    let mut spans: Vec<Span<'static>> = combined_prefix_spans(prefix, diff, bg);
    let sep_style = bg.map_or(Style::default(), |b| Style::default().bg(b));
    spans.push(Span::styled(" ", sep_style));
    let mut content_style = Style::default().fg(diff.context_fg);
    if let Some(bg) = bg {
        content_style = content_style.bg(bg);
    }
    spans.push(Span::styled(content.to_string(), content_style));
    spans
}
