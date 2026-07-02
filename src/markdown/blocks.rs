use crate::theme::MarkdownTheme;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use syntect::{highlighting::Theme, parsing::SyntaxSet};
use unicode_width::UnicodeWidthStr;

use super::latex;
use super::lists::{list_item_prefix, ItemState, ListKind};
use super::mermaid;
use super::syntax::highlight_code;
use super::toc::TocEntry;
use super::width::display_width;
use super::wrapping::{push_wrapped_code_lines, push_wrapped_prefixed_lines};
use super::LastBlock;

pub(super) struct BlockLayout {
    pub(super) prefix_width: usize,
    pub(super) rendered_width: usize,
}

pub(super) fn block_prefix(
    in_bq: bool,
    theme: &MarkdownTheme,
    marker_color: Option<Color>,
) -> Vec<Span<'static>> {
    if in_bq {
        let color = marker_color.unwrap_or(theme.blockquote_marker);
        vec![Span::styled("▏ ", Style::default().fg(color))]
    } else {
        vec![]
    }
}

pub(super) fn push_wrapped_blockquote_lines(
    lines: &mut Vec<Line<'static>>,
    body_spans: &mut Vec<Span<'static>>,
    render_width: usize,
    theme: &MarkdownTheme,
    marker_color: Option<Color>,
) {
    let prefix = block_prefix(true, theme, marker_color);
    push_wrapped_prefixed_lines(lines, body_spans, prefix.clone(), prefix, render_width);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn flush_wrapped_spans(
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    blockquote_depth: usize,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
    render_width: usize,
    theme: &MarkdownTheme,
    marker_color: Option<Color>,
) {
    if blockquote_depth > 0 && item_stack.is_empty() {
        push_wrapped_blockquote_lines(lines, spans, render_width, theme, marker_color);
    } else if !item_stack.is_empty() {
        let first_prefix = list_item_prefix(
            blockquote_depth > 0,
            list_stack,
            item_stack,
            theme,
            marker_color,
        );
        let continuation_prefix = list_item_prefix(
            blockquote_depth > 0,
            list_stack,
            item_stack,
            theme,
            marker_color,
        );
        push_wrapped_prefixed_lines(
            lines,
            spans,
            first_prefix,
            continuation_prefix,
            render_width,
        );
    } else if !spans.is_empty() {
        push_wrapped_prefixed_lines(lines, spans, vec![], vec![], render_width);
    }
}

pub(super) fn trim_paragraph_gap_before_block(
    lines: &mut Vec<Line<'static>>,
    last_block: LastBlock,
) {
    if last_block == LastBlock::Paragraph
        && lines
            .last()
            .is_some_and(|line| super::width::line_plain_text(line).is_empty())
    {
        lines.pop();
    }
}

pub(super) fn push_heading_lines(
    lines: &mut Vec<Line<'static>>,
    toc: &mut Vec<TocEntry>,
    spans: &mut Vec<Span<'static>>,
    level: u8,
    render_width: usize,
    theme: &MarkdownTheme,
) {
    let color: Color = match level {
        1 => theme.heading_1,
        2 => theme.heading_2,
        3 => theme.heading_3,
        4 => theme.heading_4,
        _ => theme.heading_other,
    };
    let modifier = match level {
        1..=5 => Modifier::BOLD,
        _ => Modifier::ITALIC,
    };
    let heading_style = Style::default().fg(color).add_modifier(modifier);
    let title: String = spans.iter().map(|s| s.content.as_ref()).collect();
    toc.push(TocEntry {
        level,
        title: title.clone(),
        line: lines.len(),
    });
    let styled_spans: Vec<Span<'static>> = spans
        .drain(..)
        .map(|span| {
            let mut style = heading_style;
            if span.style.bg.is_some() {
                style.fg = span.style.fg;
                style.bg = span.style.bg;
                style.sub_modifier = modifier;
            } else if span.style.fg == Some(theme.link_text)
                || span.style.fg == Some(theme.link_icon)
            {
                style.fg = span.style.fg;
            }
            Span::styled(span.content, style)
        })
        .collect();
    lines.push(Line::from(styled_spans));

    match level {
        1 => lines.push(Line::from(Span::styled(
            "═".repeat(display_width(&title).min(super::rule_width(render_width, 0))),
            Style::default().fg(theme.heading_underline),
        ))),
        2 => lines.push(Line::from(Span::styled(
            "─".repeat(display_width(&title).min(super::rule_width(render_width, 0))),
            Style::default().fg(theme.heading_underline),
        ))),
        _ => {}
    }
}

pub(super) const CODE_BLOCK_GUTTER: &str = "│";
pub(super) const CODE_BLOCK_GUTTER_WRAP: &str = "┊";

pub(super) struct CodeBlockRenderContext<'a> {
    pub(super) ss: &'a SyntaxSet,
    pub(super) theme: &'a Theme,
    pub(super) render_width: usize,
    pub(super) theme_colors: &'a MarkdownTheme,
    pub(super) blockquote_depth: usize,
    pub(super) list_stack: &'a [ListKind],
    pub(super) file_mode: bool,
    pub(super) code_line_numbers: bool,
}

pub(super) fn push_code_block_lines(
    lines: &mut Vec<Line<'static>>,
    code_buf: &mut String,
    code_lang: &mut String,
    ctx: CodeBlockRenderContext<'_>,
    item_stack: &mut [ItemState],
) -> BlockLayout {
    let prefix = if !item_stack.is_empty() {
        list_item_prefix(
            ctx.blockquote_depth > 0,
            ctx.list_stack,
            item_stack,
            ctx.theme_colors,
            None,
        )
    } else if ctx.blockquote_depth > 0 {
        block_prefix(true, ctx.theme_colors, None)
    } else {
        Vec::new()
    };
    let prefix_width: usize = prefix
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum();
    let label = if code_lang.is_empty() {
        "text".to_string()
    } else {
        code_lang.clone()
    };
    let show_line_numbers = ctx.code_line_numbers;
    let available_width = ctx.render_width.saturating_sub(prefix_width);
    let (code_lines, inner_width, digit_width) = highlight_code(
        code_buf,
        code_lang,
        ctx.ss,
        ctx.theme,
        available_width,
        ctx.file_mode,
        show_line_numbers,
    );
    let gutter_width = if show_line_numbers {
        digit_width + 2
    } else {
        1
    };
    let gutter_style = Style::default().fg(ctx.theme_colors.code_gutter);
    let content_width = inner_width.saturating_sub(gutter_width + 1);

    let header_width = UnicodeWidthStr::width(label.as_str()) + 3;
    let top_bar = "─".repeat(inner_width.saturating_sub(header_width));
    let mut header = prefix.clone();
    header.extend([
        Span::styled(
            "┌─ ".to_string(),
            Style::default().fg(ctx.theme_colors.code_frame),
        ),
        Span::styled(
            format!("{label} "),
            Style::default().fg(ctx.theme_colors.code_label),
        ),
        Span::styled(
            format!("{top_bar}┐"),
            Style::default().fg(ctx.theme_colors.code_frame),
        ),
    ]);
    lines.push(Line::from(header));

    let plain_first_gutter = Span::styled(CODE_BLOCK_GUTTER, gutter_style);
    let plain_cont_gutter = Span::styled(CODE_BLOCK_GUTTER_WRAP, gutter_style);

    for (i, code_line) in code_lines.into_iter().enumerate() {
        let (first_gutter, cont_gutter) = if show_line_numbers {
            let line_num = i + 1;
            (
                Span::styled(format!("│{:>w$}│", line_num, w = digit_width), gutter_style),
                Span::styled(format!("│{:>w$}│", "", w = digit_width), gutter_style),
            )
        } else {
            (plain_first_gutter.clone(), plain_cont_gutter.clone())
        };

        let mut first_prefix = prefix.clone();
        first_prefix.push(first_gutter);

        let mut cont_prefix = prefix.clone();
        cont_prefix.push(cont_gutter);

        push_wrapped_code_lines(
            lines,
            code_line.content_spans,
            first_prefix,
            cont_prefix,
            gutter_style,
            content_width,
        );
    }

    let mut footer = prefix;
    let footer_text = if show_line_numbers {
        format!(
            "└{}┴{}┘",
            "─".repeat(gutter_width - 2),
            "─".repeat(inner_width.saturating_sub(gutter_width - 1))
        )
    } else {
        format!("└{}┘", "─".repeat(inner_width))
    };
    footer.push(Span::styled(
        footer_text,
        Style::default().fg(ctx.theme_colors.code_frame),
    ));
    lines.push(Line::from(footer));
    code_lang.clear();
    code_buf.clear();
    BlockLayout {
        prefix_width,
        rendered_width: inner_width + 2,
    }
}

pub(super) struct SpecialBlockCtx<'a, F: Fn(&str) -> Vec<Span<'static>>> {
    pub(super) label: &'a str,
    pub(super) content_lines: &'a [&'a str],
    pub(super) show_line_numbers: bool,
    pub(super) center: bool,
    pub(super) make_spans: F,
}

pub(super) fn push_special_block_lines<F: Fn(&str) -> Vec<Span<'static>>>(
    lines: &mut Vec<Line<'static>>,
    render_width: usize,
    theme: &MarkdownTheme,
    blockquote_depth: usize,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
    ctx: SpecialBlockCtx<'_, F>,
) -> BlockLayout {
    let label = ctx.label;
    let content_lines = ctx.content_lines;
    let show_line_numbers = ctx.show_line_numbers;
    let center = ctx.center;
    let prefix = if !item_stack.is_empty() {
        list_item_prefix(blockquote_depth > 0, list_stack, item_stack, theme, None)
    } else if blockquote_depth > 0 {
        block_prefix(true, theme, None)
    } else {
        Vec::new()
    };
    let prefix_width: usize = prefix
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum();
    let available_width = render_width.saturating_sub(prefix_width);
    let frame_style = Style::default().fg(theme.code_frame);
    let label_style = Style::default().fg(theme.code_label);
    let gutter_style = Style::default().fg(theme.code_gutter);

    let total_lines = content_lines.len().max(1);
    let (digit_width, gutter_width) = if show_line_numbers {
        let dw = total_lines.to_string().len();
        (dw, dw + 2)
    } else {
        (0, 1)
    };

    let max_text = content_lines
        .iter()
        .map(|l| display_width(l))
        .max()
        .unwrap_or(0);
    let max_inner_width = available_width
        .saturating_sub(2)
        .max(UnicodeWidthStr::width(label) + 3);
    let min_inner = (UnicodeWidthStr::width(label) + 3)
        .max(44)
        .min(max_inner_width);
    let inner_width = (max_text + 2 + gutter_width)
        .max(min_inner)
        .min(max_inner_width);
    let content_width = inner_width.saturating_sub(gutter_width + 1);

    let header_width = UnicodeWidthStr::width(label) + 3;
    let top_bar = "─".repeat(inner_width.saturating_sub(header_width));
    let mut header = prefix.clone();
    header.extend([
        Span::styled("┌─ ".to_string(), frame_style),
        Span::styled(format!("{label} "), label_style),
        Span::styled(format!("{top_bar}┐"), frame_style),
    ]);
    lines.push(Line::from(header));

    let center_pad = if center {
        " ".repeat(content_width.saturating_sub(max_text) / 2)
    } else {
        String::new()
    };
    let plain_first_gutter = Span::styled(CODE_BLOCK_GUTTER, gutter_style);
    let plain_cont_gutter = Span::styled(CODE_BLOCK_GUTTER_WRAP, gutter_style);

    for (i, content_line) in content_lines.iter().enumerate() {
        let mut content_spans = (ctx.make_spans)(content_line);
        if !center_pad.is_empty() {
            content_spans.insert(0, Span::raw(center_pad.clone()));
        }

        let mut first_prefix = prefix.clone();
        let mut cont_prefix = prefix.clone();
        if !show_line_numbers {
            first_prefix.push(plain_first_gutter.clone());
            cont_prefix.push(plain_cont_gutter.clone());
        } else {
            let line_num = i + 1;
            first_prefix.push(Span::styled(
                format!("│{:>w$}│", line_num, w = digit_width),
                gutter_style,
            ));
            cont_prefix.push(Span::styled(
                format!("│{:>w$}│", "", w = digit_width),
                gutter_style,
            ));
        }

        push_wrapped_code_lines(
            lines,
            content_spans,
            first_prefix,
            cont_prefix,
            gutter_style,
            content_width,
        );
    }

    let mut footer = prefix;
    if show_line_numbers {
        footer.push(Span::styled(
            format!(
                "└{}┴{}┘",
                "─".repeat(gutter_width - 2),
                "─".repeat(inner_width.saturating_sub(gutter_width - 1))
            ),
            frame_style,
        ));
    } else {
        footer.push(Span::styled(
            format!("└{}┘", "─".repeat(inner_width)),
            frame_style,
        ));
    }
    lines.push(Line::from(footer));
    BlockLayout {
        prefix_width,
        rendered_width: inner_width + 2,
    }
}

pub(super) struct EmbeddedBlockCtx<'a> {
    pub(super) render_width: usize,
    pub(super) theme: &'a MarkdownTheme,
    pub(super) blockquote_depth: usize,
    pub(super) list_stack: &'a [ListKind],
    pub(super) code_line_numbers: bool,
}

pub(super) fn push_latex_block_lines(
    lines: &mut Vec<Line<'static>>,
    content: &str,
    ctx: EmbeddedBlockCtx<'_>,
    item_stack: &mut [ItemState],
) -> BlockLayout {
    let rendered = latex::to_unicode(content);
    let all_lines: Vec<&str> = rendered.lines().collect();
    let start = all_lines
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(0);
    let end = all_lines
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map_or(start, |e| e + 1);
    let content_style = Style::default().fg(ctx.theme.latex_block_fg);
    push_special_block_lines(
        lines,
        ctx.render_width,
        ctx.theme,
        ctx.blockquote_depth,
        ctx.list_stack,
        item_stack,
        SpecialBlockCtx {
            label: "latex",
            content_lines: &all_lines[start..end],
            show_line_numbers: ctx.code_line_numbers,
            center: false,
            make_spans: |line| vec![Span::styled(line.to_string(), content_style)],
        },
    )
}

pub(super) fn push_mermaid_block_lines(
    lines: &mut Vec<Line<'static>>,
    content: &str,
    ctx: EmbeddedBlockCtx<'_>,
    item_stack: &mut [ItemState],
) -> BlockLayout {
    let rendered = mermaid::render(content);
    let use_rendered = rendered.is_some();
    let content_lines: Vec<&str> = if let Some(ref r) = rendered {
        r.lines().collect()
    } else {
        content.lines().collect()
    };
    let content_style = Style::default().fg(ctx.theme.mermaid_block_fg);
    push_special_block_lines(
        lines,
        ctx.render_width,
        ctx.theme,
        ctx.blockquote_depth,
        ctx.list_stack,
        item_stack,
        SpecialBlockCtx {
            label: "mermaid",
            content_lines: &content_lines,
            show_line_numbers: !use_rendered && ctx.code_line_numbers,
            center: use_rendered,
            make_spans: |line| {
                if use_rendered {
                    vec![Span::styled(line.to_string(), content_style)]
                } else {
                    mermaid::colorize_line(line, ctx.theme)
                }
            },
        },
    )
}

pub(super) fn push_rule_line(
    lines: &mut Vec<Line<'static>>,
    render_width: usize,
    theme: &MarkdownTheme,
) {
    lines.push(Line::from(Span::styled(
        "─".repeat(super::rule_width(render_width, 0)),
        Style::default().fg(theme.rule),
    )));
    lines.push(Line::from(""));
}
