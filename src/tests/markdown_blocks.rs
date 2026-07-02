use super::{rendered_non_empty_lines, test_assets, test_md_theme};
use crate::markdown::{parse_markdown, parse_markdown_with_width};
use crate::*;

#[test]
fn h1_headings_render_double_rule_without_bottom_spacing() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) =
        parse_markdown("# 東京\n", &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered[0], "東京");
    assert_eq!(rendered[1], "═".repeat(display_width("東京")));
}

#[test]
fn paragraph_and_following_code_block_have_no_blank_gap() {
    let (ss, theme) = test_assets();
    let src = "Intro paragraph\n\n```rs\nfn main() {}\n```\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();
    let intro_idx = rendered
        .iter()
        .position(|line| line == "Intro paragraph")
        .unwrap();

    assert!(rendered[intro_idx + 1].starts_with("┌─ rs "));
}

#[test]
fn nested_blockquotes_keep_quote_prefix_after_inner_quote_ends() {
    let (ss, theme) = test_assets();
    let src = "> outer\n> > inner\n> outer again\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line == "▏ outer"));
    assert!(rendered.iter().any(|line| line == "▏ inner"));
    assert!(rendered.iter().any(|line| line == "▏ outer again"));
}

#[test]
fn long_blockquotes_wrap_into_multiple_prefixed_lines() {
    let (ss, theme) = test_assets();
    let src = "> This is a long blockquote line that should wrap into multiple quoted lines at narrow widths.\n";
    let (lines, _, _, _) =
        parse_markdown_with_width(src, &ss, &theme, 28, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let quoted: Vec<_> = rendered
        .into_iter()
        .filter(|line| line.starts_with('▏'))
        .collect();

    assert!(quoted.len() >= 2);
    assert!(quoted.iter().all(|line| line.starts_with("▏ ")));
}

#[test]
fn frontmatter_is_ignored_in_preview() {
    let (ss, theme) = test_assets();
    let src = "---\ntitle: Demo\nowner: me\n---\n# Visible\nBody\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(!rendered.iter().any(|line| line.contains("title: Demo")));
    assert!(rendered.iter().any(|line| line.contains("Visible")));
}

#[test]
fn h2_headings_are_underlined_and_compact() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown_with_width(
        "Intro\n\n## Section\nBody\n",
        &ss,
        &theme,
        40,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line.contains("Section")));
    assert!(rendered.iter().any(|line| line.contains("────")));
}

#[test]
fn rules_use_render_width_without_extra_blank_after() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown_with_width(
        "Alpha\n\n---\nBeta\n",
        &ss,
        &theme,
        24,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered = rendered_non_empty_lines(&lines);
    let rule = rendered
        .iter()
        .find(|line| line.trim_start().starts_with('─'))
        .unwrap();

    assert_eq!(display_width(rule.trim_start()), 24);
    let rule_idx = rendered.iter().position(|line| line == rule).unwrap();
    assert_eq!(rendered[rule_idx + 1], "Beta");
}

#[test]
fn body_dashes_are_thematic_breaks_not_metadata_block() {
    let (ss, theme) = test_assets();
    let src = "# Title\n\n---\n### SSL\n\n- item 1\n- item 2\n\n---\n### TLS\nend\n";
    let (lines, _, _, _) =
        parse_markdown_with_width(src, &ss, &theme, 40, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    let rule_count = rendered
        .iter()
        .filter(|line| line.trim_start().starts_with('─'))
        .count();
    assert!(
        rule_count >= 2,
        "expected two thematic breaks, got {rule_count}"
    );
    assert!(rendered.iter().any(|line| line.contains("SSL")));
    assert!(rendered.iter().any(|line| line.contains("item 1")));
    assert!(rendered.iter().any(|line| line.contains("item 2")));
    assert!(rendered.iter().any(|line| line.contains("TLS")));
}

#[test]
fn source_line_map_plain_document_is_aligned_with_first_event() {
    let (ss, theme) = test_assets();
    let src = "# Title\n\nfirst paragraph\n";
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    let first = parsed.source_line_map[0];
    let last_content_idx = parsed.lines.len().saturating_sub(6);
    assert_eq!(first, 1);
    assert_eq!(parsed.source_line_map[last_content_idx], 3);
}

#[test]
fn source_line_map_skips_code_block_fence_drift() {
    let (ss, theme) = test_assets();
    let src = "intro\n\n```rs\nfn main() {}\n```\nend\n";
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    let rendered: Vec<String> = parsed.lines.iter().map(line_plain_text).collect();
    let code_row = rendered
        .iter()
        .position(|l| l.contains("fn main()"))
        .expect("code rendered");
    assert!(parsed.source_line_map[code_row] >= 3);
    let end_row = rendered
        .iter()
        .position(|l| l == "end")
        .expect("end rendered");
    assert_eq!(parsed.source_line_map[end_row], 6);
}

#[test]
fn source_line_map_frontmatter_lines_point_to_line_one() {
    let (ss, theme) = test_assets();
    let src = "---\ntitle: Hello\nauthor: Me\n---\nbody\n";
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    assert_eq!(parsed.source_line_map[0], 1);
    let rendered: Vec<String> = parsed.lines.iter().map(line_plain_text).collect();
    let body_idx = rendered
        .iter()
        .position(|l| l == "body")
        .expect("body rendered");
    assert_eq!(parsed.source_line_map[body_idx], 5);
}

#[test]
fn source_line_map_file_mode_first_content_line_is_one() {
    let (ss, theme) = test_assets();
    let wrapped = App::fence_wrap("fn main() {}\n", "rs");
    let parsed = parse_markdown(&wrapped, &ss, &theme, &test_md_theme(), true, true);
    let rendered: Vec<String> = parsed.lines.iter().map(line_plain_text).collect();
    let code_row = rendered
        .iter()
        .position(|l| l.contains("fn main()"))
        .expect("code rendered");
    assert_eq!(parsed.source_line_map[code_row], 1);
}

#[test]
fn source_line_map_padding_repeats_last_event_source_line() {
    let (ss, theme) = test_assets();
    let src = "first\n\nmiddle\n\nlast\n";
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    let total = parsed.lines.len();
    let last = parsed.source_line_map[total - 1];
    assert_eq!(last, 5);
}

#[test]
fn code_blocks_capture_raw_content_with_trailing_newline() {
    let (ss, theme) = test_assets();
    let src = "```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n";
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    assert_eq!(parsed.code_blocks.len(), 1);
    let block = &parsed.code_blocks[0];
    assert_eq!(block.raw_content, "fn main() {\n    println!(\"hi\");\n}\n");
    assert!(block.rendered_start < block.rendered_end);
    assert!(block.rendered_end < parsed.lines.len());
}

#[test]
fn code_blocks_capture_mermaid_raw_content() {
    let (ss, theme) = test_assets();
    let src = "```mermaid\ngraph TD;\nA-->B;\n```\n";
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    assert_eq!(parsed.code_blocks.len(), 1);
    let block = &parsed.code_blocks[0];
    assert_eq!(block.raw_content, "graph TD;\nA-->B;\n");
    assert!(block.rendered_end < parsed.lines.len());
}

#[test]
fn code_blocks_capture_latex_raw_content() {
    let (ss, theme) = test_assets();
    let src = "```latex\nE = mc^2\n```\n";
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    assert_eq!(parsed.code_blocks.len(), 1);
    let block = &parsed.code_blocks[0];
    assert_eq!(block.raw_content, "E = mc^2\n");
    assert!(block.rendered_end < parsed.lines.len());
}

#[test]
fn code_blocks_multiple_blocks_are_captured_in_order() {
    let (ss, theme) = test_assets();
    let src = "```rust\nfirst\n```\n\nsome text\n\n```python\nsecond\n```\n";
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    assert_eq!(parsed.code_blocks.len(), 2);
    assert_eq!(parsed.code_blocks[0].raw_content, "first\n");
    assert_eq!(parsed.code_blocks[1].raw_content, "second\n");
    assert!(parsed.code_blocks[0].rendered_end < parsed.code_blocks[1].rendered_start);
}

#[test]
fn code_blocks_raw_content_excludes_fences() {
    let (ss, theme) = test_assets();
    let src = "```\nplain code\n```\n";
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    assert_eq!(parsed.code_blocks.len(), 1);
    let block = &parsed.code_blocks[0];
    assert!(!block.raw_content.contains("```"));
    assert_eq!(block.raw_content, "plain code\n");
}

#[test]
fn code_blocks_capture_display_math_dollar_syntax() {
    let (ss, theme) = test_assets();
    let src = "$$ E = mc^2 $$\n";
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    assert_eq!(parsed.code_blocks.len(), 1);
    let block = &parsed.code_blocks[0];
    assert_eq!(block.raw_content, "E = mc^2");
    assert!(block.rendered_start <= block.rendered_end);
    assert!(block.rendered_end < parsed.lines.len());
}

#[test]
fn code_blocks_ignores_inline_math_dollar_syntax() {
    let (ss, theme) = test_assets();
    let src = "Voici une formule inline $a + b$ dans un paragraphe.\n";
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    assert_eq!(parsed.code_blocks.len(), 0);
}
