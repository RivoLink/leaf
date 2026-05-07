use super::{find_symbol, render_buffer, test_assets, test_md_theme};
use crate::markdown::parse_markdown;
use crate::one_shot::{write_lines, ColorMode};
use crate::wrap_path_lines;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

#[test]
fn code_block_box_renders_right_border_in_one_column() {
    let (ss, theme) = test_assets();
    let md = "```ts\nconst city = \"東京\";\n\tconsole.log(city)\n```";
    let (lines, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
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
fn table_render_right_border_stays_aligned() {
    let (ss, theme) = test_assets();
    let md = "| Name | Value |\n| --- | --- |\n| 東京 | 12 |\n| tab\tcell | ok |";
    let (lines, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
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
    let (lines, _) = parse_markdown(md, &ss, &theme, &test_md_theme());
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

fn plain(lines: &[ratatui::text::Line<'_>]) -> Vec<String> {
    lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        })
        .collect()
}

#[test]
fn wrap_path_lines_short_path_fits_single_line() {
    let s = Style::default();
    let lines = wrap_path_lines("Relative: ", "src/main.rs", 74, s, s);
    assert_eq!(lines.len(), 1);
    assert_eq!(plain(&lines), vec!["Relative: src/main.rs"]);
}

#[test]
fn wrap_path_lines_long_path_wraps_with_indent() {
    let s = Style::default();
    let label = "Absolute: ";
    let path = "a".repeat(80);
    let lines = wrap_path_lines(label, &path, 30, s, s);
    let text = plain(&lines);
    assert!(lines.len() > 1);
    assert!(text[0].starts_with("Absolute: "));
    for continuation in &text[1..] {
        assert!(
            continuation.starts_with("          "),
            "continuation should be indented by label width"
        );
    }
}

#[test]
fn wrap_path_lines_continuation_aligned_with_value_start() {
    let s = Style::default();
    let label = "Relative: ";
    let path = "x".repeat(100);
    let lines = wrap_path_lines(label, &path, 40, s, s);
    let text = plain(&lines);
    let value_width = 40 - label.len();
    assert_eq!(&text[0], &format!("Relative: {}", &path[..value_width]));
    assert_eq!(
        &text[1],
        &format!(
            "{}{}",
            " ".repeat(label.len()),
            &path[value_width..value_width * 2]
        )
    );
}

#[test]
fn wrap_path_lines_exact_fit_no_wrap() {
    let s = Style::default();
    let label = "Test: ";
    let path = "x".repeat(74 - label.len());
    let lines = wrap_path_lines(label, &path, 74, s, s);
    assert_eq!(lines.len(), 1);
}

#[test]
fn wrap_path_lines_one_char_over_wraps() {
    let s = Style::default();
    let label = "Test: ";
    let path = "x".repeat(74 - label.len() + 1);
    let lines = wrap_path_lines(label, &path, 74, s, s);
    assert_eq!(lines.len(), 2);
}

#[test]
fn one_shot_plain_output_writes_text_without_ansi() {
    let lines = vec![Line::from(vec![
        Span::raw("hello "),
        Span::styled("world", Style::default().fg(Color::Red)),
    ])];
    let mut output = Vec::new();

    write_lines(&mut output, &lines, ColorMode::Plain).unwrap();

    assert_eq!(String::from_utf8(output).unwrap(), "hello world\n");
}

#[test]
fn one_shot_ansi_output_writes_styles_and_resets() {
    let lines = vec![Line::from(vec![Span::styled(
        "hello",
        Style::default()
            .fg(Color::Rgb(1, 2, 3))
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )])];
    let mut output = Vec::new();

    write_lines(&mut output, &lines, ColorMode::Ansi).unwrap();

    assert_eq!(
        String::from_utf8(output).unwrap(),
        "\x1b[38;2;1;2;3;1;4mhello\x1b[0m\n"
    );
}

#[test]
fn one_shot_output_trims_trailing_empty_lines() {
    let lines = vec![Line::from("hello"), Line::from(""), Line::from("")];
    let mut output = Vec::new();

    write_lines(&mut output, &lines, ColorMode::Plain).unwrap();

    assert_eq!(String::from_utf8(output).unwrap(), "hello\n");
}
