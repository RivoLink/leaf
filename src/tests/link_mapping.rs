use super::{lock_theme_test_state, test_assets, test_md_theme};
use crate::app::{App, AppConfig};
use crate::markdown::parse_markdown_with_width;
use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

fn app_for(source: &str, parse_width: usize) -> App {
    let (ss, theme) = test_assets();
    let parsed = parse_markdown_with_width(
        source,
        &ss,
        &theme,
        parse_width,
        &test_md_theme(),
        false,
        true,
    );
    let mut app = App::new_with_source(
        Vec::new(),
        Vec::new(),
        AppConfig {
            filename: "mapping.md".into(),
            source: source.into(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );
    app.replace_content(parsed);
    app
}

fn draw(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| crate::render::ui(frame, app))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn hovered(app: &App, x: u16, y: u16) -> Option<(usize, usize)> {
    app.find_hovered_link(
        x,
        y,
        crate::render::CONTENT_HORIZONTAL_PADDING,
        crate::render::SCROLLBAR_WIDTH,
        app.line_number_gutter_width() as u16,
    )
}

fn destination(app: &App, x: u16, y: u16) -> Option<&str> {
    let (line, index) = hovered(app, x, y)?;
    app.link_url(app.link_spans_by_line.get(&line)?.get(index)?.link_id)
}

#[test]
fn adjacent_links_do_not_lose_boundaries_or_shift_later_destinations() {
    let _guard = lock_theme_test_state();
    for source in [
        "[A](https://a.test)[B](https://b.test)[C](https://c.test)\n\n[D](https://d.test)",
        "[A](https://a.test)\n\n[B](https://b.test)[C](https://c.test)\n\n[D](https://d.test)",
        "[A](https://a.test) [B](https://b.test) [C](https://c.test)\n\n[D](https://d.test)",
        "[A](https://a.test)\t[B](https://b.test)[C](https://c.test)\n\n[D](https://d.test)",
    ] {
        let mut app = app_for(source, 60);
        let buffer = draw(&mut app, 63, 8);
        for (label, url) in [
            ("A", "https://a.test"),
            ("B", "https://b.test"),
            ("C", "https://c.test"),
            ("D", "https://d.test"),
        ] {
            let mut found = false;
            for y in 0..7 {
                for x in 0..63 {
                    if buffer[(x, y)].symbol() == label {
                        assert_eq!(destination(&app, x, y), Some(url), "{source:?} at {x},{y}");
                        found = true;
                    }
                }
            }
            assert!(found, "fixture label {label}");
        }
    }
}

#[test]
fn wrapped_link_continuations_are_clickable_and_hover_together() {
    let _guard = lock_theme_test_state();
    let mut app = app_for(
        "[alpha beta gamma delta epsilon zeta eta theta iota](https://wrapped.test)",
        20,
    );
    let buffer = draw(&mut app, 23, 12);
    let mut label_cells = Vec::new();
    for y in 0..11 {
        for x in 1..21 {
            if buffer[(x, y)].symbol().chars().any(char::is_alphabetic) {
                assert_eq!(destination(&app, x, y), Some("https://wrapped.test"));
                label_cells.push((x, y));
            }
        }
    }
    assert!(label_cells.iter().any(|&(_, y)| y > 0));
    let &(x, y) = label_cells.last().unwrap();
    app.hovered_link = hovered(&app, x, y);
    let hovered = draw(&mut app, 23, 12);
    for (x, y) in label_cells {
        assert_eq!(
            hovered[(x, y)].fg,
            crate::theme::app_theme().markdown.link_hover
        );
    }
}

#[test]
fn table_visual_order_preserves_mouse_destination_and_excludes_borders() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("| Left | Right |\n|:---:|---:|\n| [A A A A A A A A A A A A A A A A](https://a.test) [C](https://c.test) | [B](https://b.test) |",36);
    let buffer = draw(&mut app, 39, 12);
    let mut seen = std::collections::HashSet::new();
    for y in 0..11 {
        for x in 1..37 {
            let symbol = buffer[(x, y)].symbol();
            let expected = match symbol {
                "A" => Some("https://a.test"),
                "B" => Some("https://b.test"),
                "C" => Some("https://c.test"),
                _ => None,
            };
            if let Some(url) = expected {
                assert_eq!(destination(&app, x, y), Some(url));
                seen.insert(symbol.to_string());
            }
            if symbol == "│" {
                assert_eq!(destination(&app, x, y), None);
            }
        }
    }
    assert_eq!(seen.len(), 3);
}

#[test]
fn tab_separated_hover_targets_only_the_correct_occurrence() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("[A](https://a.test)\t[B](https://b.test)", 60);
    let idle = draw(&mut app, 63, 8);
    assert_eq!(destination(&app, 4, 0), Some("https://b.test"));
    app.hovered_link = hovered(&app, 4, 0);
    let hovered = draw(&mut app, 63, 8);
    assert_eq!(hovered[(2, 0)], idle[(2, 0)]);
    assert_eq!(
        hovered[(4, 0)].fg,
        crate::theme::app_theme().markdown.link_hover
    );
    app.hovered_link = None;
    assert_eq!(draw(&mut app, 63, 8)[(4, 0)], idle[(4, 0)]);
}

#[test]
fn rendered_width_fast_path_matches_ratatui_control_and_unicode_rules() {
    use ratatui::{buffer::CellWidth, style::Style, text::Span};
    let ascii: String = (0u8..=127).map(char::from).collect();
    for text in [
        ascii.as_str(),
        "before\tlink\r\nafter",
        "\u{7f}label\u{1b}",
        "界한글 é 👩‍💻",
        "x\t\u{301}z",
    ] {
        let span = Span::raw(text);
        let expected: usize = span
            .styled_graphemes(Style::default())
            .map(|g| g.symbol.cell_width() as usize)
            .sum();
        assert_eq!(
            crate::markdown::width::rendered_span_width(&span),
            expected,
            "{text:?}"
        );
    }
}

#[test]
fn wrapped_reordered_footnotes_keep_all_mouse_regions_after_cursor_restore() {
    let _guard = lock_theme_test_state();
    use ratatui::buffer::CellWidth;
    let source = "Refs[^b] before[^a].\n\n[^a]: [甲乙甲乙甲乙甲乙甲乙甲乙](https://a.test)\n\n[^b]: [丙丁丙丁丙丁丙丁丙丁丙丁](https://b.test)";
    let mut app = app_for(source, 20);
    let buffer = draw(&mut app, 23, 24);
    let mut a_cells = 0;
    let mut b_cells = 0;
    let mut rows = std::collections::HashSet::new();
    for y in 0..23 {
        let mut x = 1;
        while x < 21 {
            let cell = &buffer[(x, y)];
            let url = match cell.symbol() {
                "甲" | "乙" => {
                    a_cells += 1;
                    Some("https://a.test")
                }
                "丙" | "丁" => {
                    b_cells += 1;
                    Some("https://b.test")
                }
                _ => None,
            };
            if let Some(url) = url {
                assert_eq!(destination(&app, x, y), Some(url));
                rows.insert(y);
            }
            if matches!(cell.symbol(), "¹" | "²" | "─") {
                assert_eq!(
                    destination(&app, x, y),
                    None,
                    "footnote prefixes/borders are not links"
                );
            }
            x += cell.symbol().cell_width().max(1);
        }
    }
    assert_eq!((a_cells, b_cells), (12, 12));
    assert!(rows.len() > 2, "fixture must wrap both labels");
}
