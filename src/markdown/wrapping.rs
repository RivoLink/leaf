use super::links::{emit_linked_line, LinkSpan, LinkedSpan};
use super::width::{display_width, iter_cluster_widths};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub(super) fn push_wrapped_prefixed_lines(
    lines: &mut Vec<Line<'static>>,
    ranges: &mut Vec<LinkSpan>,
    body_spans: &mut Vec<LinkedSpan>,
    first_prefix: Vec<Span<'static>>,
    continuation_prefix: Vec<Span<'static>>,
    render_width: usize,
) {
    if body_spans.is_empty() {
        return;
    }

    let first_prefix_width: usize = first_prefix
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum();
    let continuation_prefix_width: usize = continuation_prefix
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum();
    let max_width = render_width
        .saturating_sub(first_prefix_width.max(continuation_prefix_width))
        .max(8);

    let total_width: usize = body_spans
        .iter()
        .map(|s| display_width(s.span.content.as_ref()))
        .sum();
    if total_width <= max_width {
        let mut all = unowned(first_prefix);
        all.append(body_spans);
        emit_linked_line(lines, ranges, all);
        return;
    }

    let mut current_prefix = unowned(first_prefix);
    let mut next_prefix = unowned(continuation_prefix);
    let mut current_width = 0usize;
    let mut body_started = false;

    let push_current = |lines: &mut Vec<Line<'static>>,
                        ranges: &mut Vec<LinkSpan>,
                        current_prefix: &mut Vec<LinkedSpan>,
                        next_prefix: &mut Vec<LinkedSpan>,
                        body_started: &mut bool,
                        current_width: &mut usize| {
        if *body_started {
            let keep = current_prefix
                .iter()
                .rposition(|s| !s.span.content.is_empty())
                .map_or(0, |i| i + 1);
            let carried = current_prefix.split_off(keep);
            emit_linked_line(lines, ranges, std::mem::take(current_prefix));
            *current_prefix = next_prefix.clone();
            current_prefix.extend(carried);
            *body_started = false;
            *current_width = 0;
        }
    };

    for LinkedSpan { span, link_id } in body_spans.drain(..) {
        if span.content.is_empty() {
            current_prefix.push(LinkedSpan::new(span, link_id));
            continue;
        }
        let style = span.style;
        let mut token = String::new();
        let mut token_is_space = false;

        let mut flush_token = |token: &mut String,
                               token_is_space: bool,
                               lines: &mut Vec<Line<'static>>,
                               ranges: &mut Vec<LinkSpan>,
                               current_prefix: &mut Vec<LinkedSpan>,
                               body_started: &mut bool,
                               current_width: &mut usize| {
            if token.is_empty() {
                return;
            }

            let token_width = display_width(token);
            if token_is_space {
                let keep_styled_padding = style.bg.is_some();
                if (*body_started || keep_styled_padding)
                    && *current_width + token_width <= max_width
                {
                    current_prefix.push(LinkedSpan::new(
                        Span::styled(std::mem::take(token), style),
                        link_id,
                    ));
                    *current_width += token_width;
                    *body_started = true;
                } else {
                    token.clear();
                }
                return;
            }

            if *body_started && *current_width + token_width > max_width {
                push_current(
                    lines,
                    ranges,
                    current_prefix,
                    &mut next_prefix,
                    body_started,
                    current_width,
                );
            }

            if token_width <= max_width {
                current_prefix.push(LinkedSpan::new(
                    Span::styled(std::mem::take(token), style),
                    link_id,
                ));
                *current_width += token_width;
                *body_started = true;
                return;
            }

            let mut chunk = String::new();
            let mut chunk_width = 0usize;
            for (cluster, cluster_w) in iter_cluster_widths(token) {
                let would_overflow = if *body_started {
                    *current_width + chunk_width + cluster_w > max_width
                } else {
                    chunk_width + cluster_w > max_width
                };
                if would_overflow {
                    if !chunk.is_empty() {
                        current_prefix.push(LinkedSpan::new(
                            Span::styled(std::mem::take(&mut chunk), style),
                            link_id,
                        ));
                        *body_started = true;
                    }
                    push_current(
                        lines,
                        ranges,
                        current_prefix,
                        &mut next_prefix,
                        body_started,
                        current_width,
                    );
                    chunk_width = 0;
                }

                chunk.push_str(cluster);
                chunk_width += cluster_w;
            }

            if !chunk.is_empty() {
                current_prefix.push(LinkedSpan::new(Span::styled(chunk, style), link_id));
                *current_width += chunk_width;
                *body_started = true;
            }
            token.clear();
        };

        for ch in span.content.chars() {
            let is_space = ch.is_whitespace();
            if token.is_empty() {
                token_is_space = is_space;
            } else if token_is_space != is_space {
                flush_token(
                    &mut token,
                    token_is_space,
                    lines,
                    ranges,
                    &mut current_prefix,
                    &mut body_started,
                    &mut current_width,
                );
                token_is_space = is_space;
            }
            token.push(ch);
        }

        flush_token(
            &mut token,
            token_is_space,
            lines,
            ranges,
            &mut current_prefix,
            &mut body_started,
            &mut current_width,
        );
    }

    if body_started {
        emit_linked_line(lines, ranges, current_prefix);
    }
}

fn unowned(spans: Vec<Span<'static>>) -> Vec<LinkedSpan> {
    spans
        .into_iter()
        .map(|s| LinkedSpan::new(s, None))
        .collect()
}

pub(super) fn push_wrapped_code_lines(
    lines: &mut Vec<Line<'static>>,
    content_spans: Vec<Span<'static>>,
    first_prefix: Vec<Span<'static>>,
    continuation_prefix: Vec<Span<'static>>,
    suffix_style: Style,
    available_content_width: usize,
) {
    let mut clusters: Vec<(String, usize, Style)> = Vec::new();
    for span in &content_spans {
        let style = span.style;
        for (cluster, cluster_w) in iter_cluster_widths(&span.content) {
            clusters.push((cluster.to_string(), cluster_w, style));
        }
    }

    if clusters.is_empty() {
        let pad = " ".repeat(available_content_width + 1);
        let mut row = first_prefix;
        row.push(Span::raw(format!(" {pad}")));
        row.push(Span::styled("│", suffix_style));
        lines.push(Line::from(row));
        return;
    }

    let max_w = available_content_width.max(1);
    let mut pos = 0;
    let mut first_prefix = Some(first_prefix);

    while pos < clusters.len() {
        let prefix = first_prefix
            .take()
            .unwrap_or_else(|| continuation_prefix.clone());

        let row_start = pos;
        let mut row_width = 0;

        while pos < clusters.len() {
            let cluster_w = clusters[pos].1;
            if row_width + cluster_w > max_w && row_width > 0 {
                break;
            }
            row_width += cluster_w;
            pos += 1;
        }

        let mut row = prefix;
        row.push(Span::raw(" "));

        let mut current_style: Option<Style> = None;
        let mut current_text = String::new();
        for (cluster, _, st) in &clusters[row_start..pos] {
            if current_style == Some(*st) {
                current_text.push_str(cluster);
            } else {
                if !current_text.is_empty() {
                    row.push(Span::styled(
                        std::mem::take(&mut current_text),
                        current_style.unwrap(),
                    ));
                }
                current_style = Some(*st);
                current_text.push_str(cluster);
            }
        }
        if !current_text.is_empty() {
            row.push(Span::styled(current_text, current_style.unwrap()));
        }

        let pad = max_w.saturating_sub(row_width);
        row.push(Span::raw(" ".repeat(pad + 1)));
        row.push(Span::styled("│", suffix_style));
        lines.push(Line::from(row));
    }
}
