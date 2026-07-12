use super::{rendered_non_empty_lines, test_assets, test_md_theme};
use crate::line_plain_text;
use crate::markdown::{parse_markdown, parse_markdown_with_width};

#[test]
fn code_block_after_blockquote_in_list_item_has_no_blank_gap_before() {
    let (ss, theme) = test_assets();
    let src = "- header\n  > quote first\n  ```\n  code after quote\n  ```\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    let quote_idx = rendered
        .iter()
        .position(|l| l == "  ▏ quote first")
        .expect("missing blockquote line");
    let header_idx = rendered
        .iter()
        .position(|l| l.contains("┌─"))
        .expect("missing code block header");
    assert_eq!(
        header_idx,
        quote_idx + 1,
        "expected code block header directly after blockquote line, got {rendered:?}"
    );
}

#[test]
fn blockquote_after_list_item_paragraph_has_no_blank_gap_before() {
    let (ss, theme) = test_assets();
    let src = "1. first item\n   > quote\n\n2. second item\n   > another quote\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    let first_idx = rendered
        .iter()
        .position(|l| l == "1. first item")
        .expect("missing '1. first item'");
    let quote_idx = rendered
        .iter()
        .position(|l| l == "   ▏ quote")
        .expect("missing blockquote line");
    assert_eq!(
        quote_idx,
        first_idx + 1,
        "expected blockquote directly under the item content, got {rendered:?}"
    );
}

#[test]
fn code_block_in_list_item_has_same_layout_loose_and_tight() {
    let (ss, theme) = test_assets();
    let loose_md = "- one\n\n  ```\n  three\n  ```\n";
    let tight_md = "- one\n  ```\n  three\n  ```\n";
    let (loose_lines, _, _, _) =
        parse_markdown(loose_md, &ss, &theme, &test_md_theme(), false, true).into();
    let (tight_lines, _, _, _) =
        parse_markdown(tight_md, &ss, &theme, &test_md_theme(), false, true).into();
    let loose = rendered_non_empty_lines(&loose_lines);
    let tight = rendered_non_empty_lines(&tight_lines);

    let loose_one = loose
        .iter()
        .position(|l| l.contains("one"))
        .expect("missing 'one' in loose rendering");
    let loose_three = loose
        .iter()
        .position(|l| l.contains("three"))
        .expect("missing 'three' in loose rendering");
    let tight_one = tight
        .iter()
        .position(|l| l.contains("one"))
        .expect("missing 'one' in tight rendering");
    let tight_three = tight
        .iter()
        .position(|l| l.contains("three"))
        .expect("missing 'three' in tight rendering");

    assert_eq!(
        loose_three - loose_one,
        tight_three - tight_one,
        "loose and tight paths should place the code block at the same distance from the item line"
    );
}

#[test]
fn blockquote_after_continuation_line_in_tight_list_item_renders_below_it() {
    let (ss, theme) = test_assets();
    let md = "- one\n  two\n  > quote\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    let one_idx = rendered
        .iter()
        .position(|line| line.contains("one"))
        .expect("missing first list line");
    let two_idx = rendered
        .iter()
        .position(|line| line.contains("two"))
        .expect("missing continuation line");
    let quote_idx = rendered
        .iter()
        .position(|line| line.contains("quote"))
        .expect("missing blockquote line");

    assert!(
        one_idx < two_idx && two_idx < quote_idx,
        "expected blockquote below continuation line, got one={one_idx}, two={two_idx}, quote={quote_idx}"
    );
}

#[test]
fn nested_list_after_continuation_line_in_tight_list_item_renders_below_it() {
    let (ss, theme) = test_assets();
    let md = "- one\n  two\n  - nested\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    let one_idx = rendered
        .iter()
        .position(|line| line.contains("one"))
        .expect("missing first list line");
    let two_idx = rendered
        .iter()
        .position(|line| line.contains("two"))
        .expect("missing continuation line");
    let nested_idx = rendered
        .iter()
        .position(|line| line.contains("nested"))
        .expect("missing nested list line");

    assert!(
        one_idx < two_idx && two_idx < nested_idx,
        "expected nested list below continuation line, got one={one_idx}, two={two_idx}, nested={nested_idx}"
    );
}

#[test]
fn html_block_after_continuation_line_in_tight_list_item_documented_behavior() {
    let (ss, theme) = test_assets();
    let md = "- one\n  two\n  <div>html</div>\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line.contains("one")));
    assert!(rendered.iter().any(|line| line.contains("two")));
    assert!(
        !rendered.iter().any(|line| line.contains("html")),
        "documented behavior: raw HTML blocks are dropped from the rendered output"
    );
}

#[test]
fn list_items_inside_blockquote_render_bq_marker_before_list_marker() {
    let (ss, theme) = test_assets();
    let md = "> - Step one\n> - Step two\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let step_one = rendered
        .iter()
        .find(|line| line.contains("Step one"))
        .expect("missing 'Step one' line");

    let bar_idx = step_one
        .find('▏')
        .expect("expected blockquote marker in rendered line");
    let bullet_idx = step_one
        .find("• ")
        .expect("expected list marker in rendered line");
    assert!(
        bar_idx < bullet_idx,
        "blockquote marker should precede list marker when list is inside blockquote, got {step_one:?}"
    );
}

#[test]
fn list_inside_alert_places_bq_marker_before_list_marker() {
    let (ss, theme) = test_assets();
    let md = "> [!NOTE]\n> Steps to follow:\n> - Step one\n> - Step two\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let step_line = rendered
        .iter()
        .find(|line| line.contains("Step one"))
        .expect("missing 'Step one' line");

    assert!(
        step_line.starts_with("▏ • "),
        "list item inside alert should start with '▏ • ', got {step_line:?}"
    );
}

#[test]
fn alert_header_in_list_item_is_indented_under_item_content() {
    let (ss, theme) = test_assets();
    let md = "- info list item\n  > [!NOTE]\n  > This is a note inside a tight list item.\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let header = rendered
        .iter()
        .find(|line| line.contains("Note"))
        .expect("missing alert header");

    assert!(
        header.starts_with("  ▏"),
        "alert header should be indented under item content, got {header:?}"
    );
}

#[test]
fn blockquote_in_tight_list_item_is_indented_under_item_content() {
    let (ss, theme) = test_assets();
    let md = "- one\n  > quote\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let quote_line = rendered
        .iter()
        .find(|line| line.contains("quote"))
        .expect("missing blockquote line");

    assert!(
        quote_line.starts_with("  ▏"),
        "blockquote in list item should be indented under item content, got {quote_line:?}"
    );
}

#[test]
fn blockquote_after_continuation_in_tight_list_item_is_indented() {
    let (ss, theme) = test_assets();
    let md = "- one\n  two\n  > quote\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let quote_line = rendered
        .iter()
        .find(|line| line.contains("quote"))
        .expect("missing blockquote line");

    assert!(
        quote_line.starts_with("  ▏"),
        "blockquote after continuation should be indented, got {quote_line:?}"
    );
}

#[test]
fn loose_blockquote_in_list_item_is_indented_under_item_content() {
    let (ss, theme) = test_assets();
    let md = "- one\n\n  > quote\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let quote_line = rendered
        .iter()
        .find(|line| line.contains("quote"))
        .expect("missing blockquote line");

    assert!(
        quote_line.starts_with("  ▏"),
        "loose blockquote in list item should be indented, got {quote_line:?}"
    );
}

#[test]
fn blockquote_multiline_source_in_list_item_indents_all_lines() {
    let (ss, theme) = test_assets();
    let md = "- one\n  > line1\n  > line2\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let line1 = rendered
        .iter()
        .find(|line| line.contains("line1"))
        .expect("missing line1");
    let line2 = rendered
        .iter()
        .find(|line| line.contains("line2"))
        .expect("missing line2");

    assert!(
        line1.starts_with("  ▏"),
        "line1 should be indented: {line1:?}"
    );
    assert!(
        line2.starts_with("  ▏"),
        "line2 should be indented: {line2:?}"
    );
}

#[test]
fn blockquote_long_line_in_list_item_wraps_with_correct_indent() {
    let (ss, theme) = test_assets();
    let md = "- one\n  > This is a long blockquote line that should wrap into multiple prefixed lines at a narrow width.\n";
    let (lines, _, _, _) =
        parse_markdown_with_width(md, &ss, &theme, 32, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let quoted: Vec<_> = rendered.iter().filter(|line| line.contains('▏')).collect();

    assert!(
        quoted.len() >= 2,
        "expected the blockquote to wrap into multiple lines, got {quoted:?}"
    );
    assert!(
        quoted.iter().all(|line| line.starts_with("  ▏")),
        "all wrapped blockquote lines should share the list-indent prefix, got {quoted:?}"
    );
}

#[test]
fn nested_blockquote_in_list_item_uses_list_indent() {
    let (ss, theme) = test_assets();
    let md = "- > > deep\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let deep_line = rendered
        .iter()
        .find(|line| line.contains("deep"))
        .expect("missing deep line");

    assert!(
        deep_line.contains("• "),
        "list marker should be present on the item opening: {deep_line:?}"
    );
    let markers = deep_line.matches('▏').count();
    assert_eq!(
        markers, 2,
        "expected 2 blockquote markers for depth 2, got {deep_line:?}"
    );
    let bullet_idx = deep_line.find("• ").unwrap();
    let bar_idx = deep_line.find('▏').unwrap();
    assert!(
        bullet_idx < bar_idx,
        "list marker should come before blockquote markers: {deep_line:?}"
    );
}

#[test]
fn nested_blockquote_inside_list_item_uses_depth_markers() {
    let (ss, theme) = test_assets();
    let md = "- > > deep\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let deep_line = rendered
        .iter()
        .find(|line| line.contains("deep"))
        .expect("missing deep line");

    assert!(
        deep_line.contains("▏ ▏ "),
        "expected two blockquote markers in the prefix: {deep_line:?}"
    );
}

#[test]
fn ordered_list_item_with_blockquote_uses_correct_indent() {
    let (ss, theme) = test_assets();
    let md = "1. one\n   > quote\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let quote_line = rendered
        .iter()
        .find(|line| line.contains("quote"))
        .expect("missing blockquote line");

    assert!(
        quote_line.starts_with("   ▏"),
        "ordered list item should indent blockquote by 3 columns (width of '1. '), got {quote_line:?}"
    );
}

#[test]
fn nested_list_with_blockquote_uses_deeper_indent() {
    let (ss, theme) = test_assets();
    let md = "- outer\n  - inner\n    > deep quote\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let quote_line = rendered
        .iter()
        .find(|line| line.contains("deep quote"))
        .expect("missing deep quote line");

    assert!(
        quote_line.starts_with("    ▏"),
        "nested list item should indent blockquote by 4 columns (2 for outer + 2 for inner '◦ '), got {quote_line:?}"
    );
}
