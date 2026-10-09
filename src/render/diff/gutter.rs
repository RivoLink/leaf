use super::*;
use crate::{app::DiffState, theme::DiffTheme};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub(super) fn compute_split_sticky_top_idx_inner(
    file_owner: &[Option<usize>],
    file_top_idx: &[usize],
    file_bottom_idx: &[usize],
    scroll: usize,
) -> Option<usize> {
    let fi = file_owner.get(scroll).copied().flatten()?;
    let top_idx = file_top_idx.get(fi).copied()?;
    let bottom_idx = file_bottom_idx.get(fi).copied()?;
    if scroll == bottom_idx || scroll + 1 == bottom_idx {
        return None;
    }
    if top_idx >= scroll {
        return None;
    }
    Some(top_idx)
}

pub(crate) struct SplitGutterConfig<'a> {
    pub(crate) old_left: &'a [u32],
    pub(crate) new_left: &'a [u32],
    pub(crate) old_right: &'a [u32],
    pub(crate) new_right: &'a [u32],
    pub(crate) widths: (usize, usize),
    pub(crate) left_disabled: &'a [bool],
    pub(crate) right_disabled: &'a [bool],
}

pub(crate) fn apply_split_number_gutter_from_slices(
    visible_lines: &mut [Line<'static>],
    scroll: usize,
    theme: &DiffTheme,
    config: &SplitGutterConfig<'_>,
) {
    let SplitGutterConfig {
        old_left,
        new_left,
        old_right,
        new_right,
        widths,
        left_disabled,
        right_disabled,
    } = *config;
    let (digit_width_old, digit_width_new) = widths;
    let frame_style = Style::default().fg(theme.frame_fg);
    let add_style = Style::default().fg(theme.add_fg);
    let del_style = Style::default().fg(theme.del_fg);
    for (i, line) in visible_lines.iter_mut().enumerate() {
        let border_pos = line.spans.first().is_some_and(|s| s.content == "│");
        if !border_pos || line.spans.len() < 7 {
            continue;
        }

        let mid_idx = line
            .spans
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(k, s)| (s.content == "│").then_some(k));
        let Some(mid_idx) = mid_idx else { continue };

        let idx = scroll + i;
        let ol = old_left.get(idx).copied().unwrap_or(0);
        let nl = new_left.get(idx).copied().unwrap_or(0);
        let or = old_right.get(idx).copied().unwrap_or(0);
        let nr = new_right.get(idx).copied().unwrap_or(0);

        let render_gutter =
            |old: u32, new: u32| -> (String, String, Style, Style, Style, Option<Color>) {
                let old_s = if old == 0 {
                    format!("{:>w$}", "", w = digit_width_old)
                } else {
                    format!("{:>w$}", old, w = digit_width_old)
                };
                let new_s = if new == 0 {
                    format!("{:>w$}", "", w = digit_width_new)
                } else {
                    format!("{:>w$}", new, w = digit_width_new)
                };
                let (os, ns, bg) = if new > 0 && old == 0 {
                    (frame_style, add_style, Some(theme.add_bg))
                } else if old > 0 && new == 0 {
                    (del_style, frame_style, Some(theme.del_bg))
                } else {
                    (frame_style, frame_style, None)
                };
                let (os, ns, frame_bg) = if let Some(bg) = bg {
                    (os.bg(bg), ns.bg(bg), frame_style.bg(bg))
                } else {
                    (os, ns, frame_style)
                };
                (old_s, new_s, os, ns, frame_bg, bg)
            };

        let disable_left = left_disabled.get(idx).copied().unwrap_or(false);
        let disable_right = right_disabled.get(idx).copied().unwrap_or(false);

        if !disable_right {
            let (or_s, nr_s, os_r, ns_r, right_frame_style, _) = render_gutter(or, nr);
            line.spans.insert(mid_idx + 1, Span::styled(or_s, os_r));
            line.spans
                .insert(mid_idx + 2, Span::styled(" ", right_frame_style));
            line.spans.insert(mid_idx + 3, Span::styled(nr_s, ns_r));
            line.spans
                .insert(mid_idx + 4, Span::styled("│", right_frame_style));
        }
        if !disable_left {
            let (ol_s, nl_s, os_l, ns_l, left_frame_style, _) = render_gutter(ol, nl);
            line.spans.insert(1, Span::styled(ol_s, os_l));
            line.spans.insert(2, Span::styled(" ", left_frame_style));
            line.spans.insert(3, Span::styled(nl_s, ns_l));
            line.spans.insert(4, Span::styled("│", left_frame_style));
        }
    }
}

pub(crate) struct PadContext<'a> {
    pub(crate) width: u16,
    pub(crate) scroll: usize,
    pub(crate) left_nums: (&'a [u32], &'a [u32]),
    pub(crate) right_nums: (&'a [u32], &'a [u32]),
    pub(crate) has_gutter: bool,
    pub(crate) left_disabled: &'a [bool],
}

pub(crate) fn apply_split_middle_pad(
    visible_lines: &mut [Line<'static>],
    theme: &DiffTheme,
    ctx: &PadContext<'_>,
) {
    let PadContext {
        width,
        scroll,
        left_nums,
        right_nums,
        has_gutter,
        left_disabled,
    } = *ctx;
    if width < 4 {
        return;
    }
    let target = width as usize;
    let middle_col = target / 2;

    let right_end_exclusive = target.saturating_sub(1);

    let base_mid_pipe_rank: usize = if has_gutter { 2 } else { 1 };

    let (old_l_arr, new_l_arr) = left_nums;
    let (old_r_arr, new_r_arr) = right_nums;
    for (i, line) in visible_lines.iter_mut().enumerate() {
        let leads_with_border = line.spans.first().is_some_and(|s| s.content == "│");
        if !leads_with_border {
            continue;
        }

        let idx_abs = scroll + i;
        let left_absent = left_disabled.get(idx_abs).copied().unwrap_or(false);
        let mid_pipe_rank: usize = if left_absent && has_gutter {
            1
        } else {
            base_mid_pipe_rank
        };

        if line.spans.len() == 2 && line.spans[1].content == "│" {
            let dot_idx = 1;
            let left_w: usize = line.spans[..dot_idx]
                .iter()
                .map(|s| s.content.chars().count())
                .sum();
            if left_w < middle_col {
                let pad_len = middle_col - left_w;
                let pad_str: String = std::iter::repeat_n(' ', pad_len).collect();
                line.spans
                    .insert(dot_idx, Span::styled(pad_str, Style::default()));
            }
            let after_dot = if left_w < middle_col { 3 } else { 2 };
            let right_w: usize = line.spans[after_dot..]
                .iter()
                .map(|s| s.content.chars().count())
                .sum();
            let right_budget = right_end_exclusive.saturating_sub(middle_col + 1);
            if right_w < right_budget {
                let pad_len = right_budget - right_w;
                let pad_str: String = std::iter::repeat_n(' ', pad_len).collect();
                line.spans.push(Span::styled(pad_str, Style::default()));
            }
            continue;
        }

        let mid_idx = line
            .spans
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, s)| s.content == "│")
            .nth(mid_pipe_rank - 1)
            .map(|(k, _)| k);
        let Some(mid_idx) = mid_idx else { continue };

        let idx = scroll + i;
        let ol = old_l_arr.get(idx).copied().unwrap_or(0);
        let nl = new_l_arr.get(idx).copied().unwrap_or(0);
        let or = old_r_arr.get(idx).copied().unwrap_or(0);
        let nr = new_r_arr.get(idx).copied().unwrap_or(0);
        let left_bg: Option<Color> = if ol > 0 && nl == 0 {
            Some(theme.del_bg)
        } else if nl > 0 && ol == 0 {
            Some(theme.add_bg)
        } else {
            None
        };
        let right_bg: Option<Color> = if nr > 0 && or == 0 {
            Some(theme.add_bg)
        } else if or > 0 && nr == 0 {
            Some(theme.del_bg)
        } else {
            None
        };

        let left_w: usize = line.spans[..mid_idx]
            .iter()
            .map(|s| s.content.chars().count())
            .sum();

        let new_mid_idx = if left_w < middle_col {
            let pad_len = middle_col - left_w;
            let pad_str: String = std::iter::repeat_n(' ', pad_len).collect();
            let style = if let Some(bg) = left_bg {
                Style::default().bg(bg)
            } else {
                Style::default()
            };
            line.spans.insert(mid_idx, Span::styled(pad_str, style));
            mid_idx + 1
        } else if left_w > middle_col {
            truncate_left_half(&mut line.spans, mid_idx, middle_col, has_gutter, left_bg)
        } else {
            mid_idx
        };

        let right_w: usize = line.spans[(new_mid_idx + 1)..]
            .iter()
            .map(|s| s.content.chars().count())
            .sum();

        let right_budget = right_end_exclusive.saturating_sub(middle_col + 1);
        if right_w < right_budget {
            let pad_len = right_budget - right_w;
            let pad_str: String = std::iter::repeat_n(' ', pad_len).collect();
            let style = if let Some(bg) = right_bg {
                Style::default().bg(bg)
            } else {
                Style::default()
            };
            line.spans.push(Span::styled(pad_str, style));
        } else if right_w > right_budget {
            truncate_right_half(&mut line.spans, new_mid_idx, right_budget, right_bg);
        }
    }
}

pub(crate) fn compute_gutter_widths(state: &DiffState) -> (usize, usize) {
    let max_old = state
        .files
        .iter()
        .flat_map(|f| f.hunks.iter())
        .map(|h| h.header.old_start.saturating_add(h.header.old_count))
        .max()
        .unwrap_or(0);
    let max_new = state
        .files
        .iter()
        .flat_map(|f| f.hunks.iter())
        .map(|h| h.header.new_start.saturating_add(h.header.new_count))
        .max()
        .unwrap_or(0);
    let digit_width_old = max_old.max(1).to_string().len();
    let digit_width_new = max_new.max(1).to_string().len();
    (digit_width_old, digit_width_new)
}

pub(crate) fn apply_number_gutter(
    visible_lines: &mut [Line<'static>],
    state: &DiffState,
    scroll: usize,
    theme: &DiffTheme,
) {
    let (digit_width_old, digit_width_new) = compute_gutter_widths(state);
    let frame_style = Style::default().fg(theme.frame_fg);
    let add_style = Style::default().fg(theme.add_fg);
    let del_style = Style::default().fg(theme.del_fg);
    for (i, line) in visible_lines.iter_mut().enumerate() {
        let has_internal_border =
            line.spans.first().is_some_and(|s| s.content == "│") && line.spans.len() >= 3;
        if !has_internal_border {
            continue;
        }

        let is_note_row = line
            .spans
            .get(1)
            .is_some_and(|s| s.content == "!" && s.style.bg.is_some());
        if is_note_row {
            continue;
        }
        let idx = scroll + i;
        let (o, n) = (
            state.line_old_num.get(idx).copied().unwrap_or(0),
            state.line_new_num.get(idx).copied().unwrap_or(0),
        );
        let old = if o == 0 {
            format!("{:>w$}", "", w = digit_width_old)
        } else {
            format!("{:>w$}", o, w = digit_width_old)
        };
        let new = if n == 0 {
            format!("{:>w$}", "", w = digit_width_new)
        } else {
            format!("{:>w$}", n, w = digit_width_new)
        };

        let (old_style, new_style, gutter_bg) = if n > 0 && o == 0 {
            (frame_style, add_style, Some(theme.add_bg))
        } else if o > 0 && n == 0 {
            (del_style, frame_style, Some(theme.del_bg))
        } else {
            (frame_style, frame_style, None)
        };
        let (old_style, new_style, gutter_frame_style) = if let Some(bg) = gutter_bg {
            (old_style.bg(bg), new_style.bg(bg), frame_style.bg(bg))
        } else {
            (old_style, new_style, frame_style)
        };

        line.spans.insert(1, Span::styled(old, old_style));
        line.spans.insert(2, Span::styled(" ", gutter_frame_style));
        line.spans.insert(3, Span::styled(new, new_style));
        line.spans.insert(4, Span::styled("│", gutter_frame_style));
    }
}
