use super::{rendered_non_empty_lines, test_assets, test_md_theme};
use crate::markdown::{parse_markdown, parse_markdown_with_width};
use crate::*;

#[test]
fn inline_latex_renders_with_latex_style() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "The formula $x^2 + y^2$ is here.\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();

    let latex_line = lines
        .iter()
        .find(|line| line_plain_text(line).contains("x² + y²"))
        .expect("expected a line containing inline latex content");

    let latex_span = latex_line
        .spans
        .iter()
        .find(|span| span.content.contains("x² + y²"))
        .expect("expected a span with latex content");

    assert!(
        latex_span.style.bg.is_some(),
        "inline latex should have a background color"
    );
}

#[test]
fn display_latex_renders_in_framed_block() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) =
        parse_markdown("$$E = mc^2$$\n", &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("┌─ latex")),
        "expected latex block header"
    );
    assert!(
        rendered.iter().any(|line| line.contains("E = mc²")),
        "expected latex content"
    );
    assert!(
        rendered.iter().any(|line| line.contains("└")),
        "expected latex block footer"
    );
}

#[test]
fn inline_latex_is_searchable() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "Check $\\alpha + \\beta$ here.\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let searchable: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(
        searchable.iter().any(|line| line.contains("α + β")),
        "latex content should be searchable"
    );
}

#[test]
fn display_latex_in_blockquote_has_quote_prefix() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) =
        parse_markdown("> $$F = ma$$\n", &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    let header = rendered
        .iter()
        .find(|line| line.contains("┌─ latex"))
        .expect("expected latex block header in blockquote");
    assert!(
        header.starts_with('▏'),
        "latex block in blockquote should have quote prefix"
    );
}

#[test]
fn mermaid_block_renders_in_framed_block() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "```mermaid\ngraph TD\n  A --> B\n```\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("┌─ mermaid")),
        "expected mermaid block header"
    );
    assert!(
        rendered.iter().any(|line| line.contains('A')),
        "expected node A in rendered content"
    );
    assert!(
        rendered.iter().any(|line| line.contains('B')),
        "expected node B in rendered content"
    );
    assert!(
        rendered.iter().any(|line| line.contains("└")),
        "expected mermaid block footer"
    );
}

#[test]
fn oversized_horizontal_mermaid_reflows_vertically() {
    let (ss, theme) = test_assets();
    let src = "```mermaid\nflowchart LR\n  A[Collect request] --> B[Validate request] --> C[Store response]\n```\n";
    let (lines, _, _, _) =
        parse_markdown_with_width(src, &ss, &theme, 50, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().all(|line| display_width(line) <= 50),
        "responsive mermaid render should stay within the viewport: {rendered:?}"
    );
    assert!(
        rendered.iter().all(|line| !line.contains("│1│")),
        "an LR flowchart that fits vertically should remain rendered, not fall back to source"
    );
    assert!(
        rendered.iter().any(|line| line.contains("Collect request"))
            && rendered.iter().any(|line| line.contains("Store response")),
        "responsive render should preserve node labels"
    );
}

#[test]
fn oversized_mermaid_falls_back_instead_of_wrapping_diagram_rows() {
    let (ss, theme) = test_assets();
    let src = format!(
        "```mermaid\nflowchart TD\n  A[{}]\n```\n",
        "oversized ".repeat(10)
    );
    let (lines, _, _, _) =
        parse_markdown_with_width(&src, &ss, &theme, 50, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("│1│")),
        "a diagram that cannot fit in either orientation should show readable source"
    );
    assert!(
        rendered.iter().any(|line| line.contains("flowchart TD")),
        "fallback should retain the Mermaid source"
    );
}

#[test]
fn oversized_class_diagram_uses_vertical_cards() {
    let (ss, theme) = test_assets();
    let src = "```mermaid\nclassDiagram\n  class Client {\n    +BackendCapabilities capabilities\n    +from_profile()\n  }\n  class BackendAdapter {\n    <<Protocol>>\n    +invoke(OperationDef, input, deadline)\n  }\n  Client *-- BackendAdapter\n```\n";
    let (lines, _, _, _) =
        parse_markdown_with_width(src, &ss, &theme, 32, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().all(|line| display_width(line) <= 32),
        "vertical class cards should stay within the viewport: {rendered:?}"
    );
    assert!(
        rendered.iter().all(|line| !line.contains("│1│")),
        "a valid oversized class diagram should not fall back to source"
    );
    assert!(
        rendered.iter().any(|line| line.contains("BackendAdapter"))
            && rendered.iter().any(|line| line.contains("+invoke"))
            && rendered.iter().any(|line| line.contains("Client *--")),
        "vertical class rendering should preserve classes, members, and relationships"
    );
}

#[test]
fn mermaid_block_in_blockquote_has_quote_prefix() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "> ```mermaid\n> graph LR\n>   X --> Y\n> ```\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered = rendered_non_empty_lines(&lines);

    let header = rendered
        .iter()
        .find(|line| line.contains("┌─ mermaid"))
        .expect("expected mermaid block header in blockquote");
    assert!(
        header.starts_with('▏'),
        "mermaid block in blockquote should have quote prefix"
    );
}

#[test]
fn mermaid_content_is_searchable() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "```mermaid\nsequenceDiagram\n  A->>B: Hello\n```\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let searchable = markdown::width::build_searchable_lines(&lines);

    assert!(
        searchable.iter().any(|line| line.contains('A')),
        "mermaid content should be searchable (node A)"
    );
    assert!(
        searchable.iter().any(|line| line.contains("Hello")),
        "mermaid content should be searchable (message Hello)"
    );
}

#[test]
fn mermaid_rendered_block_has_no_gutter() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "```mermaid\ngraph TD\n  A --> B\n  B --> C\n```\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered = rendered_non_empty_lines(&lines);
    let content_lines: Vec<_> = rendered
        .iter()
        .filter(|l| !l.contains('┌') && !l.contains('└'))
        .collect();

    assert!(
        content_lines.iter().all(|line| !line.contains("│1│")),
        "rendered mermaid should not have numbered gutter"
    );
}

#[test]
fn mermaid_fallback_has_numbered_gutter() {
    let (ss, theme) = test_assets();
    let src = "```mermaid\ngantt\n  title Schedule\n  section Dev\n```\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("│1│")),
        "fallback mermaid should have numbered gutter"
    );
}

#[test]
fn mermaid_pie_renders_bar_chart() {
    let (ss, theme) = test_assets();
    let src = "```mermaid\npie title Languages\n  \"Rust\" : 65\n  \"Go\" : 35\n```\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("┌─ mermaid")),
        "expected mermaid block header for pie"
    );
    assert!(
        rendered.iter().any(|line| line.contains("Languages")),
        "expected pie chart title"
    );
    assert!(
        rendered.iter().any(|line| line.contains("Rust")),
        "expected Rust label in pie chart"
    );
    assert!(
        rendered.iter().any(|line| line.contains('█')),
        "expected bar characters in pie chart"
    );
    assert!(
        rendered.iter().any(|line| line.contains('%')),
        "expected percentage in pie chart"
    );
}

#[test]
fn mermaid_unsupported_type_falls_back_to_colored_source() {
    let (ss, theme) = test_assets();
    let src = "```mermaid\ngantt\n  title Schedule\n  section Phase 1\n```\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("┌─ mermaid")),
        "expected mermaid block header"
    );
    assert!(
        rendered.iter().any(|line| line.contains("gantt")),
        "unsupported type should show source (gantt keyword)"
    );
}

#[test]
fn mermaid_empty_block_renders_without_crash() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "```mermaid\n```\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(
        rendered.iter().any(|line| line.contains("┌─ mermaid")),
        "expected mermaid block header for empty block"
    );
    assert!(
        rendered.iter().any(|line| line.contains("└")),
        "expected mermaid block footer for empty block"
    );
}
