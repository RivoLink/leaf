mod blocks;
mod fences;
mod frontmatter;
mod highlight;
mod latex;
mod links;
mod lists;
mod markers;
mod mermaid;
mod spans;
mod syntax;
mod table_layout;
mod tables;
pub(crate) mod toc;
pub(crate) mod width;
mod wrapping;

pub(crate) use highlight::highlight_line;
pub(crate) use links::LinkSpan;
pub(crate) use syntax::resolve_syntax;
use tables::{handle_table_event, start_table, TableBuf};
#[cfg(test)]
pub(crate) use width::line_plain_text;
pub(crate) use width::{build_searchable_lines, display_width, truncate_display_width};

use crate::theme::MarkdownTheme;
use pulldown_cmark::{
    BlockQuoteKind, CodeBlockKind, Event as MdEvent, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::{
    hash::{Hash, Hasher},
    io,
    path::PathBuf,
};
use toc::{normalize_toc, TocEntry};

use blocks::{
    flush_wrapped_spans, push_code_block_lines, push_heading_lines, push_latex_block_lines,
    push_mermaid_block_lines, push_rule_line, trim_paragraph_gap_before_block, BlockLayout,
    CodeBlockRenderContext, EmbeddedBlockCtx, CODE_BLOCK_GUTTER,
};
use fences::normalize_code_fences;
use links::build_link_spans;
use lists::{
    end_item, end_list, flush_list_item_spans, start_item, start_list, ItemState, ListKind,
};
#[cfg(test)]
pub(crate) use lists::{TASK_CHECKED, TASK_CHECKED_ALT, TASK_UNCHECKED};
use markers::push_custom_marker_spans;
use spans::{
    handle_inline_style_event, inline_text_style, push_inline_code_span, push_inline_latex_span,
    InlineStyleState,
};

const LINK_MARKER: &str = "#";

#[derive(Clone, Copy, PartialEq, Eq)]
enum LastBlock {
    Other,
    Paragraph,
}

pub(crate) fn hash_str(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn read_file_state(path: &PathBuf) -> Option<crate::app::FileState> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(crate::app::FileState {
        modified: metadata.modified().ok()?,
        len: metadata.len(),
    })
}

pub(crate) fn hash_file_contents(path: &PathBuf) -> io::Result<u64> {
    std::fs::read_to_string(path).map(|contents| hash_str(&contents))
}

const DEFAULT_RENDER_WIDTH: usize = 80;

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn start_heading(in_heading: &mut Option<u8>, level: HeadingLevel) {
    *in_heading = Some(heading_level(level));
}

fn end_heading(
    lines: &mut Vec<Line<'static>>,
    toc: &mut Vec<TocEntry>,
    spans: &mut Vec<Span<'static>>,
    in_heading: &mut Option<u8>,
    render_width: usize,
    theme: &MarkdownTheme,
) {
    push_heading_lines(
        lines,
        toc,
        spans,
        in_heading.unwrap_or(1),
        render_width,
        theme,
    );
    *in_heading = None;
}

fn start_code_block(
    lines: &mut Vec<Line<'static>>,
    last_block: LastBlock,
    in_code: &mut bool,
    code_buf: &mut String,
    code_lang: &mut String,
    kind: &CodeBlockKind<'_>,
) {
    trim_paragraph_gap_before_block(lines, last_block);
    *in_code = true;
    code_buf.clear();
    *code_lang = match kind {
        CodeBlockKind::Fenced(lang) => lang.to_string(),
        CodeBlockKind::Indented => String::new(),
    };
}

#[allow(clippy::too_many_arguments)]
fn end_paragraph(
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    blockquote_depth: usize,
    list_stack: &[ListKind],
    item_stack: &mut [ItemState],
    render_width: usize,
    theme: &MarkdownTheme,
    marker_color: Option<Color>,
) {
    flush_wrapped_spans(
        lines,
        spans,
        blockquote_depth,
        list_stack,
        item_stack,
        render_width,
        theme,
        marker_color,
    );
    lines.push(Line::from(""));
}

fn end_blockquote(
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    blockquote_depth: &mut usize,
    theme: &MarkdownTheme,
    marker_color: Option<Color>,
) {
    if !spans.is_empty() {
        let color = marker_color.unwrap_or(theme.blockquote_marker);
        let mut all = vec![Span::styled("▏ ", Style::default().fg(color))];
        all.append(spans);
        lines.push(Line::from(all));
    }
    *blockquote_depth = blockquote_depth.saturating_sub(1);
    lines.push(Line::from(""));
}

fn alert_icon_label(kind: BlockQuoteKind) -> (&'static str, &'static str) {
    match kind {
        BlockQuoteKind::Note => ("[i]", "Note"),
        BlockQuoteKind::Tip => ("[*]", "Tip"),
        BlockQuoteKind::Important => ("[!]", "Important"),
        BlockQuoteKind::Warning => ("[!]", "Warning"),
        BlockQuoteKind::Caution => ("[x]", "Caution"),
    }
}

fn alert_color(kind: BlockQuoteKind, theme: &MarkdownTheme) -> Color {
    match kind {
        BlockQuoteKind::Note => theme.alert_note,
        BlockQuoteKind::Tip => theme.alert_tip,
        BlockQuoteKind::Important => theme.alert_important,
        BlockQuoteKind::Warning => theme.alert_warning,
        BlockQuoteKind::Caution => theme.alert_caution,
    }
}

fn rule_width(render_width: usize, indent: usize) -> usize {
    render_width.saturating_sub(indent).max(8)
}

const CUSTOM_MARKERS: &[markers::CustomMarker] = &[markers::MARK_MARKER];

fn push_text_event(
    spans: &mut Vec<Span<'static>>,
    code_buf: &mut String,
    text: &str,
    in_code: bool,
    theme: &MarkdownTheme,
    blockquote_depth: usize,
    inline: InlineStyleState,
) {
    if in_code {
        code_buf.push_str(text);
        return;
    }

    let fallback = inline_text_style(theme, blockquote_depth, inline);
    push_custom_marker_spans(text, CUSTOM_MARKERS, fallback, theme, spans);
}

#[derive(Clone, Debug)]
pub(crate) struct CodeBlockInfo {
    pub(crate) rendered_start: usize,
    pub(crate) rendered_end: usize,
    pub(crate) rendered_offset: usize,
    pub(crate) rendered_width: usize,
    pub(crate) raw_content: String,
}

fn record_code_block(
    code_blocks: &mut Vec<CodeBlockInfo>,
    raw_content: String,
    rendered_start: usize,
    rendered_end: usize,
    layout: BlockLayout,
) {
    if rendered_end < rendered_start {
        return;
    }
    code_blocks.push(CodeBlockInfo {
        rendered_start,
        rendered_end,
        rendered_offset: layout.prefix_width,
        rendered_width: layout.rendered_width,
        raw_content,
    });
}

pub(crate) struct ParseResult {
    pub(crate) lines: Vec<Line<'static>>,
    pub(crate) toc: Vec<TocEntry>,
    pub(crate) link_spans: Vec<LinkSpan>,
    pub(crate) line_number_map: Vec<usize>,
    pub(crate) source_line_map: Vec<usize>,
    pub(crate) code_blocks: Vec<CodeBlockInfo>,
}

impl ParseResult {
    pub(crate) fn preview(lines: Vec<Line<'static>>, toc: Vec<TocEntry>) -> Self {
        Self {
            lines,
            toc,
            link_spans: Vec::new(),
            line_number_map: Vec::new(),
            source_line_map: Vec::new(),
            code_blocks: Vec::new(),
        }
    }
}

#[cfg(test)]
impl From<ParseResult> for (Vec<Line<'static>>, Vec<TocEntry>, Vec<LinkSpan>, Vec<usize>) {
    fn from(p: ParseResult) -> Self {
        (p.lines, p.toc, p.link_spans, p.line_number_map)
    }
}

pub(crate) fn parse_markdown(
    src: &str,
    ss: &syntect::parsing::SyntaxSet,
    theme: &syntect::highlighting::Theme,
    md_theme: &MarkdownTheme,
    file_mode: bool,
    code_line_numbers: bool,
) -> ParseResult {
    parse_markdown_with_width(
        src,
        ss,
        theme,
        DEFAULT_RENDER_WIDTH,
        md_theme,
        file_mode,
        code_line_numbers,
    )
}

pub(crate) fn parse_markdown_with_width(
    src: &str,
    ss: &syntect::parsing::SyntaxSet,
    theme: &syntect::highlighting::Theme,
    render_width: usize,
    theme_colors: &MarkdownTheme,
    file_mode: bool,
    code_line_numbers: bool,
) -> ParseResult {
    let original_src = src;
    let (src, fm_pairs) = frontmatter::extract_frontmatter(src);
    let fm_byte_count = original_src.len() - src.len();
    let fm_line_count = original_src[..fm_byte_count].matches('\n').count();
    let file_mode_offset = if file_mode { 1usize } else { 0 };

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut state = LineMapState::new();

    if let Some(ref pairs) = fm_pairs {
        let vertical = frontmatter::is_vertical(pairs);
        let tb = TableBuf::from_key_value_pairs(pairs, vertical);
        lines.extend(tb.render(render_width));
        state.mark_all_new(lines.len());
    }
    let mut toc: Vec<TocEntry> = Vec::new();

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut in_heading: Option<u8> = None;
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();
    let mut code_blocks: Vec<CodeBlockInfo> = Vec::new();
    let mut blockquote_depth = 0usize;
    let mut inline = InlineStyleState::default();
    let mut list_stack: Vec<ListKind> = Vec::new();
    let mut item_stack: Vec<ItemState> = Vec::new();
    let mut table: Option<TableBuf> = None;
    let mut last_block = LastBlock::Other;
    let mut link_urls: Vec<String> = Vec::new();
    let mut blockquote_color: Option<Color> = None;

    let normalized = normalize_code_fences(src);
    let line_starts = compute_line_starts(&normalized);
    let parser_options = Options::all()
        - Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        - Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS;
    for (ev, range) in Parser::new_ext(&normalized, parser_options).into_offset_iter() {
        state.truncate(lines.len());
        let before = lines.len();
        let raw_line = byte_to_line(&line_starts, range.start);
        state.current_src_line = (raw_line + fm_line_count)
            .saturating_sub(file_mode_offset)
            .max(1);
        if table.is_some()
            && handle_table_event(&mut table, &ev, &mut lines, render_width, &mut link_urls)
        {
            if lines.len() > before {
                state.mark_table_lines(lines.len(), &lines);
            } else {
                state.mark_all_new(lines.len());
            }
            continue;
        }
        if handle_inline_style_event(
            &ev,
            &mut inline,
            &mut spans,
            theme_colors,
            blockquote_depth,
            &mut link_urls,
        ) {
            continue;
        }

        let mut wraps = false;
        match ev {
            MdEvent::Start(Tag::Table(aligns)) => {
                start_table(&mut table, &aligns);
            }
            MdEvent::Start(Tag::Heading { level, .. }) => {
                start_heading(&mut in_heading, level);
            }
            MdEvent::End(TagEnd::Heading(_)) => {
                end_heading(
                    &mut lines,
                    &mut toc,
                    &mut spans,
                    &mut in_heading,
                    render_width,
                    theme_colors,
                );
                last_block = LastBlock::Other;
            }
            MdEvent::Start(Tag::Paragraph) => {}
            MdEvent::End(TagEnd::Paragraph) => {
                end_paragraph(
                    &mut lines,
                    &mut spans,
                    blockquote_depth,
                    &list_stack,
                    &mut item_stack,
                    render_width,
                    theme_colors,
                    blockquote_color,
                );
                wraps = true;
                last_block = LastBlock::Paragraph;
            }
            MdEvent::Start(Tag::CodeBlock(kind)) => {
                start_code_block(
                    &mut lines,
                    last_block,
                    &mut in_code,
                    &mut code_buf,
                    &mut code_lang,
                    &kind,
                );
                last_block = LastBlock::Other;
            }
            MdEvent::End(TagEnd::CodeBlock) => {
                in_code = false;
                let raw_content = code_buf.clone();
                let rendered_start = lines.len();
                let layout = if code_lang == "latex" || code_lang == "tex" {
                    let layout = push_latex_block_lines(
                        &mut lines,
                        &code_buf,
                        EmbeddedBlockCtx {
                            render_width,
                            theme: theme_colors,
                            blockquote_depth,
                            list_stack: &list_stack,
                            code_line_numbers,
                        },
                        &mut item_stack,
                    );
                    code_buf.clear();
                    code_lang.clear();
                    wraps = true;
                    layout
                } else if code_lang == "mermaid" {
                    let layout = push_mermaid_block_lines(
                        &mut lines,
                        &code_buf,
                        EmbeddedBlockCtx {
                            render_width,
                            theme: theme_colors,
                            blockquote_depth,
                            list_stack: &list_stack,
                            code_line_numbers,
                        },
                        &mut item_stack,
                    );
                    code_buf.clear();
                    code_lang.clear();
                    layout
                } else {
                    let layout = push_code_block_lines(
                        &mut lines,
                        &mut code_buf,
                        &mut code_lang,
                        CodeBlockRenderContext {
                            ss,
                            theme,
                            render_width,
                            theme_colors,
                            blockquote_depth,
                            list_stack: &list_stack,
                            file_mode,
                            code_line_numbers,
                        },
                        &mut item_stack,
                    );
                    wraps = true;
                    layout
                };
                record_code_block(
                    &mut code_blocks,
                    raw_content,
                    rendered_start,
                    lines.len().saturating_sub(1),
                    layout,
                );
                lines.push(Line::from(""));
                last_block = LastBlock::Other;
            }
            MdEvent::Code(text) => {
                push_inline_code_span(&mut spans, text.as_ref(), theme_colors);
            }
            MdEvent::Start(Tag::BlockQuote(kind)) => {
                blockquote_depth += 1;
                if let Some(k) = kind {
                    let color = alert_color(k, theme_colors);
                    blockquote_color = Some(color);
                    let (icon, label) = alert_icon_label(k);
                    lines.push(Line::from(vec![
                        Span::styled("▏ ", Style::default().fg(color)),
                        Span::styled(
                            format!("{icon} {label}"),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
            }
            MdEvent::End(TagEnd::BlockQuote(_)) => {
                end_blockquote(
                    &mut lines,
                    &mut spans,
                    &mut blockquote_depth,
                    theme_colors,
                    blockquote_color.take(),
                );
                wraps = true;
                last_block = LastBlock::Other;
            }
            MdEvent::Start(Tag::List(start)) => {
                if !item_stack.is_empty() && !spans.is_empty() {
                    flush_list_item_spans(
                        &mut lines,
                        &mut spans,
                        &list_stack,
                        &mut item_stack,
                        blockquote_depth,
                        render_width,
                        theme_colors,
                        blockquote_color,
                    );
                    wraps = true;
                }
                start_list(&mut lines, last_block, &mut list_stack, start);
                last_block = LastBlock::Other;
            }
            MdEvent::End(TagEnd::List(_)) => {
                end_list(&mut lines, &mut list_stack);
                last_block = LastBlock::Other;
            }
            MdEvent::Start(Tag::Item) => {
                start_item(&mut item_stack);
            }
            MdEvent::End(TagEnd::Item) => {
                end_item(
                    &mut lines,
                    &mut spans,
                    &mut list_stack,
                    &mut item_stack,
                    blockquote_depth,
                    render_width,
                    theme_colors,
                    blockquote_color,
                );
                wraps = true;
                last_block = LastBlock::Other;
            }
            MdEvent::Rule => {
                push_rule_line(&mut lines, render_width, theme_colors);
                last_block = LastBlock::Other;
            }
            MdEvent::Text(text) => {
                push_text_event(
                    &mut spans,
                    &mut code_buf,
                    text.as_ref(),
                    in_code,
                    theme_colors,
                    blockquote_depth,
                    inline,
                );
            }
            MdEvent::SoftBreak | MdEvent::HardBreak if !in_code => {
                flush_wrapped_spans(
                    &mut lines,
                    &mut spans,
                    blockquote_depth,
                    &list_stack,
                    &mut item_stack,
                    render_width,
                    theme_colors,
                    blockquote_color,
                );
                wraps = true;
            }
            MdEvent::SoftBreak | MdEvent::HardBreak => {}
            MdEvent::InlineMath(text) => {
                push_inline_latex_span(&mut spans, text.as_ref(), theme_colors);
            }
            MdEvent::DisplayMath(text) => {
                if !spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut spans)));
                }
                trim_paragraph_gap_before_block(&mut lines, last_block);
                let raw_content = text.as_ref().trim().to_string();
                let rendered_start = lines.len();
                let layout = push_latex_block_lines(
                    &mut lines,
                    text.as_ref(),
                    EmbeddedBlockCtx {
                        render_width,
                        theme: theme_colors,
                        blockquote_depth,
                        list_stack: &list_stack,
                        code_line_numbers,
                    },
                    &mut item_stack,
                );
                record_code_block(
                    &mut code_blocks,
                    raw_content,
                    rendered_start,
                    lines.len().saturating_sub(1),
                    layout,
                );
                lines.push(Line::from(""));
                wraps = true;
                last_block = LastBlock::Other;
            }
            MdEvent::TaskListMarker(checked) => {
                if let Some(item) = item_stack.last_mut() {
                    item.checkbox = Some(checked);
                }
            }
            _ => {}
        }
        if wraps && lines.len() > before {
            state.mark_wrapped(before, lines.len(), &lines);
        } else {
            state.mark_all_new(lines.len());
        }
    }

    if !spans.is_empty() {
        lines.push(Line::from(spans));
        state.mark_all_new(lines.len());
    }
    for _ in 0..5 {
        lines.push(Line::from(""));
    }
    state.mark_all_new(lines.len());
    let link_spans = build_link_spans(&lines, &link_urls, theme_colors);
    ParseResult {
        lines,
        toc: normalize_toc(toc),
        link_spans,
        line_number_map: state.line_number_map,
        source_line_map: state.source_line_map,
        code_blocks,
    }
}

fn is_empty_line(line: &Line) -> bool {
    line.spans.is_empty() || (line.spans.len() == 1 && line.spans[0].content.is_empty())
}

struct LineMapState {
    line_number_map: Vec<usize>,
    source_line_map: Vec<usize>,
    logical: usize,
    current_src_line: usize,
}

impl LineMapState {
    fn new() -> Self {
        Self {
            line_number_map: Vec::new(),
            source_line_map: Vec::new(),
            logical: 0,
            current_src_line: 1,
        }
    }

    fn push(&mut self, is_new: bool) {
        if is_new {
            self.logical += 1;
        }
        self.line_number_map.push(self.logical);
        self.source_line_map.push(self.current_src_line);
    }

    fn mark_all_new(&mut self, to: usize) {
        while self.line_number_map.len() < to {
            self.push(true);
        }
    }

    fn mark_wrapped(&mut self, from: usize, to: usize, lines: &[Line<'_>]) {
        let start = self.line_number_map.len();
        for (i, line) in lines.iter().enumerate().take(to).skip(start) {
            let is_new = i == from
                || is_empty_line(line)
                || line.spans.iter().any(|s| {
                    let c = s.content.as_ref();
                    c.starts_with('┌')
                        || c.starts_with('└')
                        || c.starts_with('╔')
                        || c.starts_with('╚')
                        || (c.starts_with('│')
                            && c.ends_with('│')
                            && c.chars().any(|ch| ch.is_ascii_digit()))
                        || c == CODE_BLOCK_GUTTER
                });
            self.push(is_new);
        }
    }

    fn mark_table_lines(&mut self, to: usize, lines: &[Line<'_>]) {
        let mut prev_border = true;
        let start = self.line_number_map.len();
        for line in lines.iter().take(to).skip(start) {
            let border = line.spans.iter().any(|s| {
                let c = s.content.as_ref();
                c.starts_with('┌') || c.starts_with('├') || c.starts_with('╞') || c.starts_with('└')
            });
            self.push(border || is_empty_line(line) || prev_border);
            prev_border = border;
        }
    }

    fn truncate(&mut self, to: usize) {
        self.line_number_map.truncate(to);
        self.source_line_map.truncate(to);
    }
}

fn compute_line_starts(s: &str) -> Vec<usize> {
    let mut starts = Vec::with_capacity(s.len() / 32 + 1);
    starts.push(0);
    starts.extend(s.match_indices('\n').map(|(i, _)| i + 1));
    starts
}

fn byte_to_line(line_starts: &[usize], offset: usize) -> usize {
    line_starts.partition_point(|&s| s <= offset).max(1)
}
