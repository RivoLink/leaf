use super::*;
use crate::{app::DiffState, theme::DiffTheme};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};

pub(crate) fn compute_sticky_top_idx(state: &DiffState, scroll: usize) -> Option<usize> {
    let fi = state.file_owner.get(scroll).copied().flatten()?;
    let top_idx = state.file_top_idx.get(fi).copied()?;
    let bottom_idx = state.file_bottom_idx.get(fi).copied()?;
    if scroll == bottom_idx || scroll + 1 == bottom_idx {
        return None;
    }
    if top_idx >= scroll {
        return None;
    }
    Some(top_idx)
}

pub(super) fn inner_content_area(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(CONTENT_HORIZONTAL_PADDING),
        y: area.y,
        width: area
            .width
            .saturating_sub(CONTENT_HORIZONTAL_PADDING.saturating_mul(2))
            .saturating_sub(SCROLLBAR_WIDTH),
        height: area.height,
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct FrameDecorations<'a> {
    pub(crate) status_by_line: Option<&'a [Option<Vec<Span<'static>>>]>,
    pub(crate) scroll: usize,
    pub(crate) gutter_widths: Option<(usize, usize)>,
    pub(crate) line_nums: Option<(&'a [u32], &'a [u32])>,
    pub(crate) extra_bottom_tees: &'a [usize],
    pub(crate) extra_top_tees: &'a [usize],
    pub(crate) extra_middle_tees: &'a [usize],
    pub(crate) extra_middle_dotted_tees: &'a [usize],
    pub(crate) auto_tee_at_gutter_col: bool,
    pub(crate) single_column_body_rows: &'a [usize],
    pub(crate) single_column_mid_col: usize,
}

pub(crate) fn close_frame_right(
    lines: &mut Vec<Line<'static>>,
    width: u16,
    theme: &DiffTheme,
    deco: &FrameDecorations<'_>,
) {
    let FrameDecorations {
        status_by_line,
        scroll,
        gutter_widths,
        line_nums,
        extra_bottom_tees,
        extra_top_tees,
        extra_middle_tees,
        extra_middle_dotted_tees,
        auto_tee_at_gutter_col,
        single_column_body_rows,
        single_column_mid_col,
    } = *deco;
    if width < 2 {
        return;
    }
    let target = width as usize;
    let style = Style::default().fg(theme.frame_fg);

    let gutter_col: Option<usize> = gutter_widths.map(|(dw_old, dw_new)| 1 + dw_old + 1 + dw_new);

    let frame_deco = style;
    for (i, line) in lines.iter_mut().enumerate() {
        let Some(first) = line.spans.first() else {
            continue;
        };
        let Some(lead) = first.content.chars().next() else {
            continue;
        };
        let (right_char, fill_char) = match lead {
            '┌' => ('┐', '─'),
            '├' => ('│', ' '),
            '└' => ('┘', '─'),
            '│' => ('│', ' '),
            _ => continue,
        };
        let existing: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();

        if lead == '┌' {
            let status = status_by_line
                .and_then(|s| s.get(scroll + i))
                .and_then(|o| o.as_ref());
            if let Some(status_spans) = status {
                let status_width: usize =
                    status_spans.iter().map(|s| s.content.chars().count()).sum();

                let overhead = existing + 1 + status_width + 1 + 1 + 1;
                if overhead <= target {
                    let fill = target - overhead;
                    if fill > 0 {
                        let fill_start = existing;
                        let fill_end = existing + fill;
                        let mut tees: Vec<usize> = Vec::new();
                        if auto_tee_at_gutter_col {
                            if let Some(col) = gutter_col {
                                tees.push(col);
                            }
                        }
                        for col in extra_top_tees {
                            tees.push(*col);
                        }
                        tees.retain(|c| *c >= fill_start && *c < fill_end);
                        tees.sort_unstable();
                        tees.dedup();
                        if tees.is_empty() {
                            line.spans.push(Span::styled("─".repeat(fill), frame_deco));
                        } else {
                            let mut cursor = fill_start;
                            for col in &tees {
                                let before = col - cursor;
                                if before > 0 {
                                    line.spans
                                        .push(Span::styled("─".repeat(before), frame_deco));
                                }
                                line.spans.push(Span::styled("┬", frame_deco));
                                cursor = col + 1;
                            }
                            let after = fill_end - cursor;
                            if after > 0 {
                                line.spans.push(Span::styled("─".repeat(after), frame_deco));
                            }
                        }
                    }
                    line.spans.push(Span::styled(" ", frame_deco));
                    for s in status_spans {
                        line.spans.push(s.clone());
                    }
                    line.spans.push(Span::styled(" ", frame_deco));
                    line.spans.push(Span::styled("─", style));
                    line.spans.push(Span::styled(right_char.to_string(), style));
                } else {
                    line.spans.push(Span::styled(" ", frame_deco));
                    for s in status_spans {
                        line.spans.push(s.clone());
                    }
                    line.spans.push(Span::styled(" ", frame_deco));
                    line.spans.push(Span::styled(right_char.to_string(), style));
                }
                continue;
            }
        }

        if existing + 1 > target {
            continue;
        }
        let pad = target - 1 - existing;

        let default_connector: Option<char> = match lead {
            '└' => Some('┴'),
            '┌' => Some('┬'),
            '├' => Some('┼'),
            _ => None,
        };
        if let Some(default_ch) = default_connector {
            let extra: &[usize] = match lead {
                '└' => extra_bottom_tees,
                '┌' => extra_top_tees,
                '├' => extra_middle_tees,
                _ => &[],
            };

            let mut tees: Vec<(usize, char)> = Vec::new();
            if auto_tee_at_gutter_col {
                if let Some(col) = gutter_col {
                    tees.push((col, default_ch));
                }
            }
            for col in extra {
                tees.push((*col, default_ch));
            }
            if lead == '├' {
                for col in extra_middle_dotted_tees {
                    tees.push((*col, '│'));
                }
            }

            tees.retain(|(c, _)| *c > existing && *c < existing + pad);
            tees.sort_by_key(|(c, _)| *c);
            let mut deduped: Vec<(usize, char)> = Vec::with_capacity(tees.len());
            for (c, ch) in tees {
                if let Some(last) = deduped.last_mut() {
                    if last.0 == c {
                        last.1 = ch;
                        continue;
                    }
                }
                deduped.push((c, ch));
            }
            if !deduped.is_empty() {
                let mut cursor = existing;
                for (col, ch) in &deduped {
                    let before = col - cursor;
                    let mut seg = String::with_capacity(before * fill_char.len_utf8());
                    for _ in 0..before {
                        seg.push(fill_char);
                    }
                    line.spans.push(Span::styled(seg, style));
                    line.spans.push(Span::styled(ch.to_string(), style));
                    cursor = col + 1;
                }
                let after = existing + pad - cursor;
                let mut trail =
                    String::with_capacity(after * fill_char.len_utf8() + right_char.len_utf8());
                for _ in 0..after {
                    trail.push(fill_char);
                }
                trail.push(right_char);
                line.spans.push(Span::styled(trail, style));
                continue;
            }
        }

        let body_bg: Option<Color> = if lead == '│' {
            let span_bg = line.spans.get(1).and_then(|s| s.style.bg);
            span_bg.or_else(|| {
                line_nums.and_then(|(old_nums, new_nums)| {
                    let idx = scroll + i;
                    let o = old_nums.get(idx).copied().unwrap_or(0);
                    let n = new_nums.get(idx).copied().unwrap_or(0);
                    if n > 0 && o == 0 {
                        Some(theme.add_bg)
                    } else if o > 0 && n == 0 {
                        Some(theme.del_bg)
                    } else {
                        None
                    }
                })
            })
        } else {
            None
        };
        let needs_mid_splice = lead == '│'
            && single_column_mid_col > 0
            && single_column_body_rows.contains(&(scroll + i));
        let pad_style = body_bg.map_or(style, |bg| Style::default().bg(bg));

        if needs_mid_splice {
            let fill_start = existing;
            let fill_end = existing + pad;
            if single_column_mid_col > fill_start && single_column_mid_col < fill_end {
                let before = single_column_mid_col - fill_start;
                let after = fill_end - (single_column_mid_col + 1);
                if before > 0 {
                    let head: String = std::iter::repeat_n(fill_char, before).collect();
                    line.spans.push(Span::styled(head, pad_style));
                }
                line.spans.push(Span::styled("│", style));
                if after > 0 {
                    let tail: String = std::iter::repeat_n(fill_char, after).collect();

                    line.spans.push(Span::styled(tail, style));
                }
                line.spans.push(Span::styled(right_char.to_string(), style));
            } else if pad > 0 {
                let pad_str: String = std::iter::repeat_n(fill_char, pad).collect();
                line.spans.push(Span::styled(pad_str, pad_style));
                line.spans.push(Span::styled(right_char.to_string(), style));
            } else {
                line.spans.push(Span::styled(right_char.to_string(), style));
            }
        } else if body_bg.is_some() {
            if pad > 0 {
                let pad_str: String = std::iter::repeat_n(fill_char, pad).collect();
                line.spans.push(Span::styled(pad_str, pad_style));
            }
            line.spans.push(Span::styled(right_char.to_string(), style));
        } else {
            let mut tail = String::with_capacity(pad + right_char.len_utf8());
            for _ in 0..pad {
                tail.push(fill_char);
            }
            tail.push(right_char);
            line.spans.push(Span::styled(tail, style));
        }
    }
}
