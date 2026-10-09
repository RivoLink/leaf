use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use super::links::{LinkId, LinkedSpan};
use super::tables::{CellFragment, CellInlineStyle};
use super::width::{display_width, expand_tabs, iter_cluster_widths};
use super::with_link_marker;

pub(super) fn fragments_display_width(frags: &[CellFragment]) -> usize {
    frags.iter().map(|f| f.display_width()).sum()
}

pub(super) fn min_table_cell_width(frags: &[CellFragment]) -> usize {
    let mut max_width = 4usize;
    for frag in frags {
        let w = if frag.is_text() {
            frag.rendered_text()
                .split_whitespace()
                .map(display_width)
                .max()
                .unwrap_or(0)
                .min(12)
        } else {
            display_width(&frag.rendered_text()).min(12) + 2
        };
        max_width = max_width.max(w);
    }
    max_width
}

fn table_available_width(col_count: usize, render_width: usize) -> usize {
    let border_width = 3 * col_count + 1;
    render_width.saturating_sub(border_width).max(col_count)
}

pub(super) fn fit_table_widths(
    col_widths: &mut [usize],
    min_widths: &[usize],
    render_width: usize,
) {
    if col_widths.is_empty() {
        return;
    }

    let col_count = col_widths.len();
    let available = table_available_width(col_count, render_width);
    let min_total: usize = min_widths.iter().sum();

    if min_total >= available {
        let mut widths = vec![1; col_count];
        let mut remaining = available.saturating_sub(col_count);
        let mut order: Vec<usize> = (0..col_count).collect();
        order.sort_by_key(|&idx| std::cmp::Reverse(min_widths[idx]));
        for idx in order {
            if remaining == 0 {
                break;
            }
            let extra = (min_widths[idx].saturating_sub(1)).min(remaining);
            widths[idx] += extra;
            remaining -= extra;
        }
        col_widths.copy_from_slice(&widths);
        return;
    }

    while col_widths.iter().sum::<usize>() > available {
        let Some((idx, _)) = col_widths
            .iter()
            .enumerate()
            .filter(|(idx, width)| **width > min_widths[*idx])
            .max_by_key(|(_, width)| **width)
        else {
            break;
        };
        col_widths[idx] -= 1;
    }
}

pub(super) fn cap_table_widths(col_widths: &mut [usize], render_width: usize) {
    let col_count = col_widths.len();
    if col_count == 0 {
        return;
    }
    let available = table_available_width(col_count, render_width);
    let max_col = (available / col_count).max(1);

    let need: Vec<usize> = col_widths.to_vec();
    for w in col_widths.iter_mut() {
        *w = (*w).min(max_col);
    }

    let surplus = available.saturating_sub(col_widths.iter().sum());
    let demand_count = need
        .iter()
        .zip(col_widths.iter())
        .filter(|(n, w)| n > w)
        .count();
    if surplus == 0 || demand_count == 0 {
        return;
    }
    let share = surplus / demand_count;
    let rem = surplus % demand_count;
    let mut k = 0;
    for i in 0..col_count {
        if need[i] > col_widths[i] {
            let want = need[i] - col_widths[i];
            col_widths[i] += (share + usize::from(k < rem)).min(want);
            k += 1;
        }
    }
}

fn rebuild_fragment(frag: &CellFragment, text: String) -> CellFragment {
    let link_id = frag.link_id();
    match frag {
        CellFragment::InlineMath(_, _, _) => CellFragment::InlineMath(text, false, link_id),
        CellFragment::Mark(_, _, _) => CellFragment::Mark(text, false, link_id),
        _ => CellFragment::Code(text, false, link_id),
    }
}

fn break_row(lines: &mut Vec<Vec<CellFragment>>, current_line: &mut Vec<CellFragment>) -> usize {
    let marker = match current_line.last() {
        Some(CellFragment::LinkMarker(_)) if current_line.len() > 1 => current_line.pop(),
        _ => None,
    };
    if marker.is_some() {
        if let Some(CellFragment::Text(t, _, _, _)) = current_line.last() {
            if t == " " {
                current_line.pop();
            }
        }
    }
    lines.push(std::mem::take(current_line));
    current_line.extend(marker);
    current_line.iter().map(CellFragment::display_width).sum()
}

pub(super) fn wrap_table_cell(frags: &[CellFragment], width: usize) -> Vec<Vec<CellFragment>> {
    if width == 0 {
        return vec![vec![]];
    }
    if frags.is_empty() {
        return vec![vec![]];
    }

    let mut lines: Vec<Vec<CellFragment>> = Vec::new();
    let mut current_line: Vec<CellFragment> = Vec::new();
    let mut current_width = 0usize;
    let mut glue = false;
    let space_frag = |line: &[CellFragment], next: Option<LinkId>| {
        let owner = line
            .last()
            .and_then(CellFragment::link_id)
            .filter(|id| Some(*id) == next);
        CellFragment::Text(" ".to_string(), CellInlineStyle::default(), false, owner)
    };

    for frag in frags {
        match frag {
            CellFragment::Text(t, style, adj, link_id) => {
                let expanded = expand_tabs(t, 0);
                let style = *style;
                let adj = *adj;
                let link_id = *link_id;
                let mut first_word = true;
                for word in expanded.split_whitespace() {
                    let word_width = display_width(word);

                    if word_width > width {
                        if !matches!(current_line.as_slice(), [] | [CellFragment::LinkMarker(_)]) {
                            current_width = break_row(&mut lines, &mut current_line);
                        }
                        glue = false;
                        first_word = false;
                        let mut chunk = String::new();
                        let mut chunk_width = current_width;
                        for (cluster, cluster_w) in iter_cluster_widths(word) {
                            if chunk_width + cluster_w > width && !chunk.is_empty() {
                                let mut row = std::mem::take(&mut current_line);
                                row.push(CellFragment::Text(
                                    std::mem::take(&mut chunk),
                                    style,
                                    false,
                                    link_id,
                                ));
                                lines.push(row);
                                chunk_width = 0;
                            }
                            chunk.push_str(cluster);
                            chunk_width += cluster_w;
                        }
                        if !chunk.is_empty() {
                            current_line.push(CellFragment::Text(chunk, style, false, link_id));
                            current_width = chunk_width;
                        }
                        continue;
                    }

                    let suppress = adj && first_word;
                    first_word = false;
                    let needs_sep = current_width > 0 && !glue && !suppress;
                    glue = false;
                    let sep = if needs_sep { 1 } else { 0 };
                    if current_width + sep + word_width > width && current_width > 0 {
                        current_width = break_row(&mut lines, &mut current_line);
                    } else if needs_sep {
                        current_line.push(space_frag(&current_line, link_id));
                        current_width += 1;
                    }
                    current_line.push(CellFragment::Text(word.to_string(), style, false, link_id));
                    current_width += word_width;
                }
            }
            CellFragment::LinkMarker(inline) => {
                let marker_width = with_link_marker(display_width);
                let sep = if current_width == 0 { 0 } else { 1 };
                if current_width + sep + marker_width > width && current_width > 0 {
                    lines.push(std::mem::take(&mut current_line));
                    current_width = 0;
                }
                if current_width > 0 {
                    current_line.push(space_frag(&current_line, inline.link_id));
                    current_width += 1;
                }
                current_line.push(frag.clone());
                current_width += marker_width;
                glue = true;
            }
            CellFragment::HardBreak => {
                lines.push(std::mem::take(&mut current_line));
                current_width = 0;
                glue = false;
            }
            CellFragment::Code(_, adj, link_id)
            | CellFragment::InlineMath(_, adj, link_id)
            | CellFragment::Mark(_, adj, link_id) => {
                let adj = *adj;
                let link_id = *link_id;
                let text = frag.rendered_text();
                let frag_width = display_width(&text) + 2;

                if frag_width > width {
                    if !current_line.is_empty() || current_width > 0 {
                        lines.push(std::mem::take(&mut current_line));
                        current_width = 0;
                    }
                    glue = false;
                    let inner = width.saturating_sub(2).max(1);
                    let mut chunk = String::new();
                    let mut chunk_width = 0usize;
                    for (cluster, cluster_w) in iter_cluster_widths(&text) {
                        if chunk_width + cluster_w > inner && !chunk.is_empty() {
                            lines.push(vec![rebuild_fragment(frag, std::mem::take(&mut chunk))]);
                            chunk_width = 0;
                        }
                        chunk.push_str(cluster);
                        chunk_width += cluster_w;
                    }
                    if !chunk.is_empty() {
                        current_line.push(rebuild_fragment(frag, chunk));
                        current_width = chunk_width + 2;
                    }
                    continue;
                }

                let sep = if current_width == 0 || adj { 0 } else { 1 };
                if current_width + sep + frag_width > width && current_width > 0 {
                    lines.push(std::mem::take(&mut current_line));
                    current_width = 0;
                }
                if current_width > 0 && !adj {
                    current_line.push(space_frag(&current_line, link_id));
                    current_width += 1;
                }
                current_line.push(frag.clone());
                current_width += frag_width;
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(vec![]);
    }
    lines
}

pub(super) fn align_cell(
    frags: &[CellFragment],
    width: usize,
    align: pulldown_cmark::Alignment,
    base_style: Style,
    is_header: bool,
    theme: &crate::theme::MarkdownTheme,
) -> Vec<LinkedSpan> {
    let mut spans = Vec::new();
    let mut content_width = 0usize;

    for frag in frags {
        match frag {
            CellFragment::Text(t, inline, _, link_id) => {
                let expanded = expand_tabs(t, 0);
                content_width += display_width(&expanded);
                let mut style = base_style;
                if inline.bold > 0 {
                    style = style.add_modifier(Modifier::BOLD);
                    if !is_header {
                        style = style.fg(theme.strong_text);
                    }
                }
                if inline.italic > 0 {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if inline.strikethrough > 0 {
                    style = style.add_modifier(Modifier::CROSSED_OUT);
                }
                if inline.underline > 0 {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                if inline.link_id.is_some() {
                    style = style.fg(theme.link_text).add_modifier(Modifier::UNDERLINED);
                }
                spans.push(LinkedSpan::new(Span::styled(expanded, style), *link_id));
            }
            CellFragment::LinkMarker(inline) => {
                with_link_marker(|marker| {
                    let style = Style::default()
                        .fg(theme.link_icon)
                        .add_modifier(inline.modifiers());
                    content_width += display_width(marker);
                    spans.push(LinkedSpan::new(
                        Span::styled(marker.to_string(), style),
                        inline.link_id,
                    ));
                });
            }
            CellFragment::HardBreak => {}
            CellFragment::Code(_, _, link_id)
            | CellFragment::InlineMath(_, _, link_id)
            | CellFragment::Mark(_, _, link_id) => {
                let styled = format!(" {} ", frag.rendered_text());
                content_width += display_width(&styled);
                let (fg, bg) = match frag {
                    CellFragment::Code(..) => (theme.inline_code_fg, theme.inline_code_bg),
                    CellFragment::Mark(..) => (theme.mark_fg, theme.mark_bg),
                    _ => (theme.latex_inline_fg, theme.latex_inline_bg),
                };
                spans.push(LinkedSpan::new(
                    Span::styled(styled, Style::default().fg(fg).bg(bg)),
                    *link_id,
                ));
            }
        }
    }

    if content_width < width {
        let pad = width - content_width;
        match align {
            pulldown_cmark::Alignment::Right => {
                spans.insert(
                    0,
                    LinkedSpan::new(Span::styled(" ".repeat(pad), base_style), None),
                );
            }
            pulldown_cmark::Alignment::Center => {
                let l = pad / 2;
                spans.insert(
                    0,
                    LinkedSpan::new(Span::styled(" ".repeat(l), base_style), None),
                );
                spans.push(LinkedSpan::new(
                    Span::styled(" ".repeat(pad - l), base_style),
                    None,
                ));
            }
            _ => {
                spans.push(LinkedSpan::new(
                    Span::styled(" ".repeat(pad), base_style),
                    None,
                ));
            }
        }
    }

    spans
}
