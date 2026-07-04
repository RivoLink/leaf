use super::{rendered_non_empty_lines, test_assets, test_md_theme};
use crate::markdown::{parse_markdown, parse_markdown_with_width};
use crate::theme::app_theme;
use crate::*;
use ratatui_image::picker::Picker;

#[test]
fn narrow_tables_fit_render_width_and_wrap_cells() {
    let (ss, theme) = test_assets();
    let md = "| Column | Description | Value |\n| --- | --- | ---: |\n| Width | Terminal-dependent layout behavior | 80 |\n";
    let (lines, _, _, _) = parse_markdown_with_width(
        md,
        &ss,
        &theme,
        36,
        &test_md_theme(),
        false,
        true,
        None,
        &Picker::halfblocks(),
    )
    .into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.len() >= 6);
    assert!(rendered.iter().all(|line| display_width(line) <= 36));
}

#[test]
fn table_inline_code_has_code_style() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| `code` |\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let app_theme = app_theme();
    let theme_colors = &app_theme.markdown;

    let has_code_span = lines.iter().any(|line| {
        line.spans.iter().any(|span| {
            span.style.bg == Some(theme_colors.inline_code_bg) && span.content.contains("code")
        })
    });
    assert!(
        has_code_span,
        "inline code in table cell should have inline_code_bg"
    );
}

#[test]
fn table_inline_code_has_padding() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| `x` |\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let app_theme = app_theme();
    let theme_colors = &app_theme.markdown;

    let has_padded_span = lines.iter().any(|line| {
        line.spans
            .iter()
            .any(|span| span.style.bg == Some(theme_colors.inline_code_bg) && span.content == " x ")
    });
    assert!(
        has_padded_span,
        "inline code in table should be padded with spaces"
    );
}

#[test]
fn table_inline_math_renders_with_latex_style() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| $\\alpha$ |\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let app_theme = app_theme();
    let theme_colors = &app_theme.markdown;

    let has_math_span = lines.iter().any(|line| {
        line.spans.iter().any(|span| {
            span.style.bg == Some(theme_colors.latex_inline_bg) && span.content.contains('α')
        })
    });
    assert!(
        has_math_span,
        "inline math in table cell should have latex_inline_bg and render Unicode"
    );
}

#[test]
fn table_mixed_text_and_code_renders_both_styles() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| hello `world` bye |\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let app_theme = app_theme();
    let theme_colors = &app_theme.markdown;

    let table_line = lines
        .iter()
        .find(|line| line.spans.iter().any(|span| span.content.contains("hello")));
    let line = table_line.expect("should find line with 'hello'");

    let has_text = line.spans.iter().any(|span| {
        span.style.fg == Some(theme_colors.table_cell) && span.content.contains("hello")
    });
    let has_code = line.spans.iter().any(|span| {
        span.style.bg == Some(theme_colors.inline_code_bg) && span.content.contains("world")
    });
    assert!(has_text, "text fragment should use table_cell style");
    assert!(has_code, "code fragment should use inline_code_bg style");
}

#[test]
fn table_without_inline_styles_renders_normally() {
    let (ss, theme) = test_assets();
    let md = "| A | B |\n|---|---|\n| one | two |\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    assert!(rendered.iter().any(|line| line.contains("one")));
    assert!(rendered.iter().any(|line| line.contains("two")));
}

#[test]
fn table_inline_code_col_width_includes_padding() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| `longcode` |\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    let top_border = rendered.iter().find(|l| l.contains('┌')).unwrap();
    let cell_line = rendered.iter().find(|l| l.contains("longcode")).unwrap();
    let top_width = display_width(top_border);
    let cell_width = display_width(cell_line);
    assert_eq!(
        top_width, cell_width,
        "border and cell lines should have same width"
    );
}

#[test]
fn table_code_adjacent_text_no_extra_space() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| `code`:text |\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let cell_line = rendered
        .iter()
        .find(|l| l.contains("code") && l.contains(":text"))
        .expect("should find line with code and :text");
    assert!(
        !cell_line.contains("  :text"),
        "no extra space before :text — got: {cell_line}"
    );
}

#[test]
fn table_bold_adjacent_text_no_extra_space() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| **bold**:text |\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let cell_line = rendered
        .iter()
        .find(|l| l.contains("bold") && l.contains(":text"))
        .expect("should find line with bold and :text");
    assert!(
        !cell_line.contains(" :text"),
        "no space before :text — got: {cell_line}"
    );
}

#[test]
fn table_apostrophe_no_split() {
    let (ss, theme) = test_assets();
    let md = "| A |\n|---|\n| apos'trophe |\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);
    let cell_line = rendered
        .iter()
        .find(|l| l.contains("apos"))
        .expect("should find line with apos");
    assert!(
        !cell_line.contains(" \u{2019} "),
        "no spaces around smart apostrophe — got: {cell_line}"
    );
}

#[test]
fn table_long_inline_code_wraps_without_clipping() {
    let (ss, theme) = test_assets();
    let code = "verylonginlinecodethatcannotfitinasinglenarrowcolumn";
    let md = format!("| Description |\n|---|\n| `{code}` |\n");
    let width = 24;
    let (lines, _, _, _) = parse_markdown_with_width(
        &md,
        &ss,
        &theme,
        width,
        &test_md_theme(),
        false,
        true,
        None,
        &Picker::halfblocks(),
    )
    .into();
    let rendered = rendered_non_empty_lines(&lines);
    let theme_colors = &app_theme().markdown;

    assert!(
        rendered.iter().all(|line| display_width(line) <= width),
        "table lines should not exceed render width"
    );

    let code_lines = lines
        .iter()
        .filter(|line| {
            line.spans
                .iter()
                .any(|span| span.style.bg == Some(theme_colors.inline_code_bg))
        })
        .count();
    assert!(
        code_lines >= 2,
        "long inline code should wrap across multiple lines, got {code_lines}"
    );

    let reassembled: String = lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .filter(|span| span.style.bg == Some(theme_colors.inline_code_bg))
        .map(|span| span.content.trim())
        .collect();
    assert_eq!(
        reassembled, code,
        "wrapped code must preserve all characters"
    );
}
