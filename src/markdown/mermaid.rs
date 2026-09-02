use crate::theme::MarkdownTheme;
use mmdflux::{render_diagram, OutputFormat, RenderConfig};
use ratatui::{style::Style, text::Span};
use std::fmt::Write;

use super::width::{display_width, truncate_display_width};

pub(crate) fn render(content: &str, max_width: usize) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("pie") {
        return render_pie(trimmed).filter(|rendered| fits_width(rendered, max_width));
    }

    let rendered = render_diagram(trimmed, OutputFormat::Text, &RenderConfig::default()).ok();
    if rendered
        .as_deref()
        .is_some_and(|rendered| fits_width(rendered, max_width))
    {
        return rendered;
    }

    if trimmed.starts_with("classDiagram") {
        // Only substitute the compact renderer for a valid diagram that is too
        // wide. Parse/render failures should retain the source fallback.
        rendered.as_ref()?;
        if let Some(horizontal) = use_horizontal_class_direction(trimmed) {
            if let Ok(rendered) =
                render_diagram(&horizontal, OutputFormat::Text, &RenderConfig::default())
            {
                if fits_width(&rendered, max_width) {
                    return Some(rendered);
                }
            }
        }
        return render_vertical_class_diagram(trimmed, max_width);
    }

    let vertical = use_vertical_direction(trimmed)?;
    render_diagram(&vertical, OutputFormat::Text, &RenderConfig::default())
        .ok()
        .filter(|rendered| fits_width(rendered, max_width))
}

fn fits_width(rendered: &str, max_width: usize) -> bool {
    max_width > 0
        && rendered
            .lines()
            .all(|line| display_width(line) <= max_width)
}

fn use_vertical_direction(content: &str) -> Option<String> {
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();

    for line in &mut lines {
        let leading_width = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        let Some(keyword_end) = trimmed.find(char::is_whitespace) else {
            continue;
        };
        if !matches!(&trimmed[..keyword_end], "flowchart" | "graph") {
            continue;
        }

        let after_keyword = &trimmed[keyword_end..];
        let whitespace_width = after_keyword.len() - after_keyword.trim_start().len();
        let direction_start = leading_width + keyword_end + whitespace_width;
        let direction = line[direction_start..]
            .split_whitespace()
            .next()?
            .trim_end_matches(';');
        if !matches!(direction, "LR" | "RL") {
            return None;
        }

        line.replace_range(direction_start..direction_start + direction.len(), "TD");
        return Some(lines.join("\n"));
    }

    None
}

fn use_horizontal_class_direction(content: &str) -> Option<String> {
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let header = lines
        .iter()
        .position(|line| line.trim() == "classDiagram")?;

    for line in lines.iter_mut().skip(header + 1) {
        let trimmed = line.trim();
        let Some(direction) = trimmed.strip_prefix("direction ") else {
            continue;
        };
        if matches!(direction.trim_end_matches(';'), "LR" | "RL") {
            return None;
        }
        if !matches!(direction.trim_end_matches(';'), "TB" | "TD" | "BT") {
            continue;
        }

        let direction_start = line.find(direction)?;
        let direction_len = direction.trim_end_matches(';').len();
        line.replace_range(direction_start..direction_start + direction_len, "LR");
        return Some(lines.join("\n"));
    }

    lines.insert(header + 1, "    direction LR".to_string());
    Some(lines.join("\n"))
}

#[derive(Default)]
struct ClassBlock {
    name: String,
    members: Vec<String>,
}

fn render_vertical_class_diagram(content: &str, max_width: usize) -> Option<String> {
    if max_width < 12 {
        return None;
    }

    let mut classes = Vec::new();
    let mut relationships = Vec::new();
    let mut current: Option<ClassBlock> = None;

    for source_line in content.lines().skip(1) {
        let line = source_line.trim();
        if line.is_empty() || line.starts_with("direction ") {
            continue;
        }

        if let Some(class) = current.as_mut() {
            if line == "}" {
                classes.push(current.take().unwrap());
            } else {
                class.members.push(line.to_string());
            }
            continue;
        }

        if let Some(declaration) = line.strip_prefix("class ") {
            if let Some(name) = declaration.strip_suffix('{') {
                current = Some(ClassBlock {
                    name: name.trim().to_string(),
                    members: Vec::new(),
                });
            } else {
                classes.push(ClassBlock {
                    name: declaration.trim().to_string(),
                    members: Vec::new(),
                });
            }
            continue;
        }

        if looks_like_class_relationship(line) {
            relationships.push(line.to_string());
        }
    }
    if let Some(class) = current {
        classes.push(class);
    }
    if classes.is_empty() {
        return None;
    }

    let widest_class_line = classes
        .iter()
        .flat_map(|class| std::iter::once(&class.name).chain(class.members.iter()))
        .map(|line| display_width(line))
        .max()
        .unwrap_or(0);
    let card_width = (widest_class_line + 4).clamp(12, max_width);
    let inner_width = card_width - 2;
    let text_width = inner_width.saturating_sub(2).max(1);
    let border = "─".repeat(inner_width);
    let mut out = String::new();

    for (index, class) in classes.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let _ = writeln!(out, "┌{border}┐");
        push_card_text(&mut out, &class.name, text_width, inner_width);
        if !class.members.is_empty() {
            let _ = writeln!(out, "├{border}┤");
            for member in &class.members {
                push_card_text(&mut out, member, text_width, inner_width);
            }
        }
        let _ = writeln!(out, "└{border}┘");
    }

    if !relationships.is_empty() {
        out.push('\n');
        let heading = truncate_display_width("Relationships", max_width);
        let _ = writeln!(out, "{heading}");
        let _ = writeln!(out, "{}", "─".repeat(display_width(&heading)));
        for relationship in relationships {
            push_prefixed_wrapped_text(&mut out, &relationship, "• ", "  ", max_width);
        }
    }

    Some(out.trim_end().to_string())
}

fn looks_like_class_relationship(line: &str) -> bool {
    line.split_whitespace().any(|part| {
        part.contains("--") || part.contains("..") || part.contains("<|") || part.contains("|>")
    })
}

fn push_card_text(out: &mut String, text: &str, text_width: usize, inner_width: usize) {
    for line in wrap_display_text(text, text_width) {
        let padding = inner_width.saturating_sub(display_width(&line) + 1);
        let _ = writeln!(out, "│ {line}{}│", " ".repeat(padding));
    }
}

fn push_prefixed_wrapped_text(
    out: &mut String,
    text: &str,
    first_prefix: &str,
    continuation_prefix: &str,
    max_width: usize,
) {
    let prefix_width = display_width(first_prefix).max(display_width(continuation_prefix));
    let line_width = max_width.saturating_sub(prefix_width).max(1);
    for (index, line) in wrap_display_text(text, line_width).into_iter().enumerate() {
        let prefix = if index == 0 {
            first_prefix
        } else {
            continuation_prefix
        };
        let _ = writeln!(out, "{prefix}{line}");
    }
}

fn wrap_display_text(text: &str, max_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let separator = usize::from(!current.is_empty());
        if display_width(&current) + separator + display_width(word) <= max_width {
            if separator > 0 {
                current.push(' ');
            }
            current.push_str(word);
            continue;
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }

        let mut remainder = word;
        while display_width(remainder) > max_width {
            let split = remainder
                .char_indices()
                .take_while(|(index, ch)| {
                    display_width(&remainder[..*index]) + display_width(&ch.to_string())
                        <= max_width
                })
                .map(|(index, ch)| index + ch.len_utf8())
                .last()
                .unwrap_or_else(|| remainder.chars().next().unwrap().len_utf8());
            lines.push(remainder[..split].to_string());
            remainder = &remainder[split..];
        }
        current.push_str(remainder);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn render_pie(content: &str) -> Option<String> {
    let mut title = String::new();
    let mut entries: Vec<(String, f64)> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("pie") {
            let rest = rest.trim();
            if rest.is_empty() {
                continue;
            }
            if let Some(t) = rest.strip_prefix("title") {
                title = t.trim().to_string();
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("title") {
            title = rest.trim().to_string();
            continue;
        }
        if let Some((label_part, value_part)) = line.rsplit_once(':') {
            let label = label_part.trim().trim_matches('"').to_string();
            if let Ok(value) = value_part.trim().parse::<f64>() {
                entries.push((label, value));
            }
        }
    }

    if entries.is_empty() {
        return None;
    }

    let total: f64 = entries.iter().map(|(_, v)| *v).sum();
    if total <= 0.0 {
        return None;
    }

    let max_label_width = entries.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
    let bar_max = 32;
    let mut out = String::new();

    if !title.is_empty() {
        let _ = writeln!(out, "{title}");
    }

    for (label, value) in &entries {
        let pct = value / total * 100.0;
        let bar_units = pct / 100.0 * bar_max as f64;
        let filled = bar_units as usize;
        let half = (bar_units * 2.0) as usize % 2 == 1;
        let bar: String = "█".repeat(filled) + if half { "▌" } else { "" };
        let _ = writeln!(
            out,
            "{bar:<bw$} {label:<lw$} {pct:>5.1}%",
            bw = bar_max + 1,
            lw = max_label_width,
        );
    }

    Some(out)
}

pub(crate) fn colorize_line(line: &str, theme: &MarkdownTheme) -> Vec<Span<'static>> {
    let keyword_style = Style::default().fg(theme.mermaid_keyword);
    let arrow_style = Style::default().fg(theme.mermaid_arrow);
    let label_style = Style::default().fg(theme.mermaid_label);
    let default_style = Style::default().fg(theme.mermaid_block_fg);

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut rest = line;

    while !rest.is_empty() {
        if let Some(pos) = rest.find('|') {
            let before = &rest[..pos];
            if !before.is_empty() {
                tokenize_segment(
                    before,
                    keyword_style,
                    arrow_style,
                    default_style,
                    &mut spans,
                );
            }
            let after_pipe = &rest[pos + 1..];
            if let Some(end) = after_pipe.find('|') {
                let label_content = &after_pipe[..end];
                spans.push(Span::styled(format!("|{label_content}|"), label_style));
                rest = &after_pipe[end + 1..];
            } else {
                spans.push(Span::styled("|".to_string(), default_style));
                rest = after_pipe;
            }
            continue;
        }

        tokenize_segment(rest, keyword_style, arrow_style, default_style, &mut spans);
        break;
    }

    if spans.is_empty() {
        spans.push(Span::styled(line.to_string(), default_style));
    }

    spans
}

fn tokenize_segment(
    segment: &str,
    keyword_style: Style,
    arrow_style: Style,
    default_style: Style,
    spans: &mut Vec<Span<'static>>,
) {
    let mut i = 0;
    let bytes = segment.as_bytes();

    while i < bytes.len() {
        if let Some((arrow, len)) = try_match_arrow(&segment[i..]) {
            spans.push(Span::styled(arrow, arrow_style));
            i += len;
            continue;
        }

        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
            {
                i += 1;
            }
            let word = &segment[start..i];
            if is_keyword(word) {
                spans.push(Span::styled(word.to_string(), keyword_style));
            } else {
                spans.push(Span::styled(word.to_string(), default_style));
            }
            continue;
        }

        let start = i;
        while i < segment.len() {
            let b = bytes[i];
            if b.is_ascii_alphabetic() || b == b'_' || b == b'|' {
                break;
            }
            if try_match_arrow(&segment[i..]).is_some() {
                break;
            }
            if b < 0x80 {
                i += 1;
            } else {
                let ch = segment[i..].chars().next().unwrap();
                i += ch.len_utf8();
            }
        }
        if i > start {
            spans.push(Span::styled(segment[start..i].to_string(), default_style));
        }
    }
}

fn try_match_arrow(s: &str) -> Option<(String, usize)> {
    for pattern in &["-.->", "==>", "-->", "---", "-.-", "-..", "->", "--"] {
        if s.starts_with(pattern) {
            return Some((pattern.to_string(), pattern.len()));
        }
    }
    None
}

fn is_keyword(word: &str) -> bool {
    is_diagram_keyword(word) || is_direction_keyword(word) || is_structure_keyword(word)
}

fn is_diagram_keyword(word: &str) -> bool {
    matches!(
        word,
        "flowchart"
            | "graph"
            | "sequenceDiagram"
            | "classDiagram"
            | "stateDiagram"
            | "stateDiagram-v2"
            | "erDiagram"
            | "gantt"
            | "pie"
            | "journey"
            | "gitGraph"
            | "mindmap"
            | "timeline"
            | "sankey-beta"
            | "quadrantChart"
            | "requirementDiagram"
            | "C4Context"
            | "block-beta"
            | "xychart-beta"
            | "kanban"
            | "architecture-beta"
    )
}

fn is_direction_keyword(word: &str) -> bool {
    matches!(word, "TB" | "TD" | "BT" | "LR" | "RL")
}

fn is_structure_keyword(word: &str) -> bool {
    matches!(
        word,
        "subgraph"
            | "end"
            | "section"
            | "title"
            | "participant"
            | "actor"
            | "loop"
            | "alt"
            | "else"
            | "opt"
            | "par"
            | "critical"
            | "break"
            | "rect"
            | "note"
            | "activate"
            | "deactivate"
            | "class"
            | "state"
            | "dateFormat"
            | "axisFormat"
            | "style"
            | "classDef"
            | "click"
    )
}
