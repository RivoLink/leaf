use ratatui::{
    style::{Color, Style},
    text::Span,
};

pub(super) fn truncate_left_half(
    spans: &mut Vec<Span<'static>>,
    mid_idx: usize,
    middle_col: usize,
    has_gutter: bool,
    bg: Option<Color>,
) -> usize {
    let preserve_end = if has_gutter { 5 } else { 1 };
    if mid_idx <= preserve_end {
        return mid_idx;
    }
    let frame_w: usize = spans[..preserve_end]
        .iter()
        .map(|s| s.content.chars().count())
        .sum();
    let bg_style = |extra: Style| -> Style {
        if let Some(c) = bg {
            extra.bg(c)
        } else {
            extra
        }
    };
    if middle_col <= frame_w + 1 {
        spans.drain(preserve_end..mid_idx);
        spans.insert(preserve_end, Span::styled("…", bg_style(Style::default())));

        return preserve_end + 1;
    }

    let chars_before_ellipsis = middle_col - frame_w - 1;
    let mut used = 0usize;
    let mut cut_at = mid_idx;
    let mut boundary_keep: Option<Span<'static>> = None;
    for (k, s) in spans[preserve_end..mid_idx].iter().enumerate() {
        let w = s.content.chars().count();
        if used + w > chars_before_ellipsis {
            let keep_chars = chars_before_ellipsis - used;
            cut_at = preserve_end + k;
            if keep_chars > 0 {
                let content: &str = &s.content;
                let split_byte = content
                    .char_indices()
                    .nth(keep_chars)
                    .map(|(b, _)| b)
                    .unwrap_or(content.len());
                let head = content[..split_byte].to_string();
                boundary_keep = Some(Span::styled(head, s.style));
            }
            used += keep_chars;
            break;
        }
        used += w;
    }

    spans.drain(cut_at..mid_idx);
    let mut insert_at = cut_at;
    if let Some(kept) = boundary_keep {
        spans.insert(insert_at, kept);
        insert_at += 1;
    }

    let remain = chars_before_ellipsis.saturating_sub(used);
    if remain > 0 {
        let pad_str: String = std::iter::repeat_n(' ', remain).collect();
        spans.insert(insert_at, Span::styled(pad_str, bg_style(Style::default())));
        insert_at += 1;
    }
    spans.insert(insert_at, Span::styled("…", bg_style(Style::default())));

    insert_at + 1
}

pub(super) fn truncate_right_half(
    spans: &mut Vec<Span<'static>>,
    mid_idx: usize,
    right_budget: usize,
    bg: Option<Color>,
) {
    truncate_spans_tail(spans, mid_idx + 1, right_budget, bg);
}

pub(crate) fn truncate_spans_tail(
    spans: &mut Vec<Span<'static>>,
    preserved_end: usize,
    budget: usize,
    bg: Option<Color>,
) {
    if spans.len() <= preserved_end || budget == 0 {
        spans.drain(preserved_end..);
        return;
    }
    let bg_style = || -> Style {
        if let Some(c) = bg {
            Style::default().bg(c)
        } else {
            Style::default()
        }
    };

    let chars_before_ellipsis = budget - 1;
    let mut used = 0usize;
    let mut cut_at = spans.len();
    let mut boundary_keep: Option<Span<'static>> = None;
    for (k, s) in spans[preserved_end..].iter().enumerate() {
        let w = s.content.chars().count();
        if used + w > chars_before_ellipsis {
            let keep_chars = chars_before_ellipsis - used;
            cut_at = preserved_end + k;
            if keep_chars > 0 {
                let content: &str = &s.content;
                let split_byte = content
                    .char_indices()
                    .nth(keep_chars)
                    .map(|(b, _)| b)
                    .unwrap_or(content.len());
                let head = content[..split_byte].to_string();
                boundary_keep = Some(Span::styled(head, s.style));
            }
            used += keep_chars;
            break;
        }
        used += w;
    }
    spans.drain(cut_at..);
    if let Some(kept) = boundary_keep {
        spans.push(kept);
    }

    let remain = chars_before_ellipsis.saturating_sub(used);
    if remain > 0 {
        let pad_str: String = std::iter::repeat_n(' ', remain).collect();
        spans.push(Span::styled(pad_str, bg_style()));
    }
    spans.push(Span::styled("…", bg_style()));
}

pub(crate) fn truncate_unified_body_row(
    spans: &mut Vec<Span<'static>>,
    target_width: usize,
    bg: Option<Color>,
) {
    if target_width < 2 {
        return;
    }

    let leads_with_border = spans.first().is_some_and(|s| s.content == "│");
    if !leads_with_border || spans.len() < 3 {
        return;
    }

    let has_gutter = spans.len() >= 8 && spans.get(4).is_some_and(|s| s.content == "│");
    let preserved_end = if has_gutter { 7 } else { 3 };
    if spans.len() < preserved_end {
        return;
    }

    let preserved_width: usize = spans[..preserved_end]
        .iter()
        .map(|s| s.content.chars().count())
        .sum();

    if preserved_width + 1 >= target_width {
        return;
    }
    let budget = target_width - 1 - preserved_width;

    let tail_width: usize = spans[preserved_end..]
        .iter()
        .map(|s| s.content.chars().count())
        .sum();

    if preserved_width + tail_width < target_width {
        return;
    }
    truncate_spans_tail(spans, preserved_end, budget, bg);
}
