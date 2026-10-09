use super::{find_symbol, render_buffer, test_assets, test_md_theme};
use crate::app::App;
use crate::markdown::{parse_markdown, parse_markdown_with_width};

#[test]
fn code_block_box_renders_right_border_in_one_column() {
    let (ss, theme) = test_assets();
    let md = "```ts\nconst city = \"東京\";\n\tconsole.log(city)\n```";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let buffer = render_buffer(&lines);

    let (right_x, start_y) = find_symbol(&buffer, "┐").unwrap();
    let (_, end_y) = find_symbol(&buffer, "┘").unwrap();

    for y in start_y + 1..end_y {
        assert_eq!(
            buffer.cell((right_x, y)).unwrap().symbol(),
            "│",
            "missing code block right border at row {y}"
        );
    }
}

#[test]
fn file_mode_code_block_fills_full_render_width() {
    let (ss, theme) = test_assets();
    let render_width = 40;
    let src = App::fence_wrap("fn main() {\n    let city = \"東京\";\n}", "rs");
    let (lines, _, _, _) = parse_markdown_with_width(
        &src,
        &ss,
        &theme,
        render_width,
        &test_md_theme(),
        true,
        true,
    )
    .into();
    let buffer = render_buffer(&lines);

    assert!(find_symbol(&buffer, "┐").is_some());
    assert!(find_symbol(&buffer, "┘").is_some());
    for line in lines.iter().filter(|line| line.width() > 0) {
        assert_eq!(
            line.width(),
            render_width,
            "code block line should fill the render width"
        );
    }
}

#[test]
fn table_render_right_border_stays_aligned() {
    let (ss, theme) = test_assets();
    let md = "| Name | Value |\n| --- | --- |\n| 東京 | 12 |\n| tab\tcell | ok |";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let buffer = render_buffer(&lines);

    let (right_x, start_y) = find_symbol(&buffer, "┐").unwrap();
    let (_, end_y) = find_symbol(&buffer, "┘").unwrap();

    for y in start_y + 1..end_y {
        let symbol = buffer.cell((right_x, y)).unwrap().symbol();
        assert!(
            matches!(symbol, "│" | "┤" | "╡"),
            "unexpected table edge symbol {symbol:?} at row {y}"
        );
    }
}

#[test]
fn table_render_right_border_stays_aligned_with_emoji_cells() {
    let (ss, theme) = test_assets();
    let md = "| Critère | Note |\n| --- | --- |\n| Tests | ✅ Bonne couverture |\n| Sécurité | ⚠ Quelques points |\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();
    let buffer = render_buffer(&lines);

    let (right_x, start_y) = find_symbol(&buffer, "┐").unwrap();
    let (_, end_y) = find_symbol(&buffer, "┘").unwrap();

    for y in start_y + 1..end_y {
        let symbol = buffer.cell((right_x, y)).unwrap().symbol();
        assert!(
            matches!(symbol, "│" | "┤" | "╡"),
            "unexpected emoji-table edge symbol {symbol:?} at row {y}"
        );
    }
}
