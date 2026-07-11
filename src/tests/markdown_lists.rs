use super::{rendered_non_empty_lines, test_assets, test_md_theme};
use crate::markdown::{parse_markdown, parse_markdown_with_width};
use crate::markdown::{TASK_CHECKED, TASK_CHECKED_ALT, TASK_UNCHECKED};
use crate::*;

#[test]
fn loose_list_items_keep_their_markers() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "- first\n\n- second\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(rendered.iter().any(|line| line.contains("• first")));
    assert!(rendered.iter().any(|line| line.contains("• second")));
}

#[test]
fn ordered_lists_render_numeric_markers() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "3. third\n4. fourth\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(rendered.iter().any(|line| line.contains("3. third")));
    assert!(rendered.iter().any(|line| line.contains("4. fourth")));
}

#[test]
fn multiline_list_items_keep_marker_only_on_first_line() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "- first line\n  second line\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    let first = rendered
        .iter()
        .find(|line| line.contains("first line"))
        .unwrap();
    let second = rendered
        .iter()
        .find(|line| line.contains("second line"))
        .unwrap();

    assert!(first.contains("• first line"));
    assert!(!second.contains('•'));
    assert!(second.starts_with("  "));
}

#[test]
fn ordered_lists_preserve_non_default_start_numbers() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "7. seven\n8. eight\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(rendered.iter().any(|line| line.contains("7. seven")));
    assert!(rendered.iter().any(|line| line.contains("8. eight")));
}

#[test]
fn loose_list_items_render_expected_lines() {
    let (ss, theme) = test_assets();
    let src = "- first loose item\n\n- second loose item after a blank line\n\n- third loose item\n\n  continuation paragraph\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(
        rendered,
        vec![
            "• first loose item",
            "• second loose item after a blank line",
            "• third loose item",
            "  continuation paragraph",
        ]
    );
}

#[test]
fn ordered_loose_lists_render_expected_lines() {
    let (ss, theme) = test_assets();
    let src = "7. seventh item\n\n8. eighth item\n\n   continuation paragraph\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(
        rendered,
        vec![
            "7. seventh item",
            "8. eighth item",
            "   continuation paragraph",
        ]
    );
}

#[test]
fn ordered_lists_render_expected_lines() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "3. third item\n4. fourth item\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered, vec!["3. third item", "4. fourth item"]);
}

#[test]
fn paragraph_and_following_list_have_no_blank_gap() {
    let (ss, theme) = test_assets();
    let (lines, _, _, _) = parse_markdown(
        "Intro paragraph\n\n- first\n- second\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();
    let intro_idx = rendered
        .iter()
        .position(|line| line == "Intro paragraph")
        .unwrap();

    assert_eq!(rendered[intro_idx + 1], "• first");
}

#[test]
fn wrapped_list_items_align_continuation_under_text() {
    let (ss, theme) = test_assets();
    let src = "- First item with enough text to wrap when the terminal is narrow and show continuation alignment.\n8. Eighth item with enough text to wrap and keep numeric alignment readable.\n";
    let (lines, _, _, _) =
        parse_markdown_with_width(src, &ss, &theme, 36, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line.starts_with("• First item")));
    assert!(rendered
        .iter()
        .any(|line| line.starts_with("  ") && line.contains("terminal is narrow")));
    assert!(rendered
        .iter()
        .any(|line| line.starts_with("8. Eighth item")));
    assert!(rendered
        .iter()
        .any(|line| line.starts_with("   ") && !line.starts_with("8. ")));
}

#[test]
fn tight_nested_list_separates_parent_and_children() {
    let (ss, theme) = test_assets();
    let src = "- parent\n  - child 1\n  - child 2\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered, vec!["• parent", "  ◦ child 1", "  ◦ child 2"]);
}

#[test]
fn tight_nested_list_three_levels_uses_correct_markers() {
    let (ss, theme) = test_assets();
    let src = "- level 1\n  - level 2\n    - level 3\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered, vec!["• level 1", "  ◦ level 2", "    ▸ level 3"]);
}

#[test]
fn tight_nested_list_unordered_parent_with_ordered_children() {
    let (ss, theme) = test_assets();
    let src = "- parent\n  1. first\n  2. second\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert_eq!(rendered, vec!["• parent", "  1. first", "  2. second"]);
}

#[test]
fn tight_nested_list_multiline_parent_with_softbreak() {
    let (ss, theme) = test_assets();
    let src = "- parent line one\n  parent line two\n  - child\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line == "• parent line one"));
    assert!(rendered
        .iter()
        .any(|line| line.starts_with("  ") && line.contains("parent line two")));
    assert!(rendered.iter().any(|line| line == "  ◦ child"));
}

#[test]
fn wrapped_list_inline_code_keeps_left_padding_in_rendered_line() {
    let (ss, theme) = test_assets();
    let source = "- `leaf --theme ocean README.md` exercises wrapping inside a list item.\n";
    let (lines, _, _, _) =
        parse_markdown_with_width(source, &ss, &theme, 22, &test_md_theme(), false, true).into();

    let target = lines
        .iter()
        .find(|line| line_plain_text(line).contains("leaf --theme"))
        .expect("expected wrapped inline-code line");

    assert!(
        target
            .spans
            .iter()
            .any(|span| span.style.bg.is_some() && span.content.starts_with(' ')),
        "expected a background-styled span with left padding"
    );
}

#[test]
fn task_markers_have_uniform_width() {
    assert_eq!(display_width(TASK_UNCHECKED), 2);
    assert_eq!(display_width(TASK_CHECKED_ALT), 2);
    assert_eq!(display_width(TASK_CHECKED), 2);
}

#[test]
fn code_block_inside_list_item_is_indented_and_has_no_blank_gap_before() {
    let (ss, theme) = test_assets();
    let md = "To put a code block within a list item, the code block needs\nto be indented *twice* -- 8 spaces or two tabs:\n\n*   A list item with a code block:\n\n        <code goes here>\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    let item_idx = rendered
        .iter()
        .position(|line| line.contains("A list item with a code block:"))
        .expect("missing list item line");
    let header_idx = rendered
        .iter()
        .position(|line| line.contains("┌─ text"))
        .expect("missing code block header");
    let code_idx = rendered
        .iter()
        .position(|line| line.contains("<code goes here>"))
        .expect("missing code line");

    assert_eq!(
        header_idx,
        item_idx + 1,
        "expected no blank gap before code block"
    );
    assert!(rendered[header_idx].starts_with("  "));
    assert!(rendered[code_idx].starts_with("  "));
}

#[test]
fn code_block_after_continuation_line_in_tight_list_item_renders_below_it() {
    let (ss, theme) = test_assets();
    let md = "- one\n  two\n  ```\n  three\n  ```\n";
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
    let three_idx = rendered
        .iter()
        .position(|line| line.contains("three"))
        .expect("missing code line");

    assert!(
        one_idx < two_idx && two_idx < three_idx,
        "expected code block below continuation line, got one={one_idx}, two={two_idx}, three={three_idx}"
    );
    assert_eq!(
        two_idx,
        one_idx + 1,
        "expected continuation line directly after first line"
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
