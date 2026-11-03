use super::common::app_helpers::{
    draw_app as draw, markdown_app, parse_markdown_for_test, sync_app_width,
};
use super::{lock_theme_test_state, test_assets};
use crate::app::App;
use ratatui::{backend::TestBackend, Terminal};

fn app_for(source: &str, parse_width: usize) -> App {
    markdown_app(source, parse_width, "mapping.md", None)
}

fn destination(app: &App, x: u16, y: u16) -> Option<&str> {
    let span = app.link_at_position(x, y)?;
    app.link_url(span.link_id)
}

const DOC: &str = "# Intro 甲乙 [Top](https://top.test)

Paragraph with [a long wrapped link label that spans rows](https://wrap.test) and then [next](https://next.test) after it, plus more words to wrap around.

## everything everything everything everything everything [Late label here](https://late.test)

[Below](https://below.test)

```
code
```

## Download https://github.com/RivoLink/leaf/releases/download/v1.29.0/leaf-markdown-viewer-aarch64-apple-darwin.tar.gz

- item [甲乙丙丁戊己庚辛壬癸](https://wide.test) tail
- ｶﾞｶﾞｶﾞｶﾞｶﾞｶﾞｶﾞｶﾞｶﾞｶﾞ [Kana](https://kana.test)

> quote with 👩‍💻👩‍💻👩‍💻 [Emoji](https://emoji.test) text

a\tb\tc\t[Tab](https://tab.test)

| Left | Right |
|:---:|---:|
| [A A A A A A A A](https://a.test) [C](https://c.test) | [B](https://b.test) |

Refs[^n] [Last](https://last.test).

[^n]: [丙丁丙丁丙丁丙丁](https://fn.test)
";

fn sync_width(app: &mut App, width: u16, height: u16) {
    draw(app, width, height);
    sync_app_width(app, width);
}

fn inner_columns(app: &App) -> (u16, u16) {
    let area = app.content_area;
    let left = area.x + crate::render::CONTENT_HORIZONTAL_PADDING;
    let right =
        area.right() - crate::render::CONTENT_HORIZONTAL_PADDING - crate::render::SCROLLBAR_WIDTH;
    (left, right)
}

fn assert_clicks_match_drawn_links(
    app: &mut App,
    width: u16,
    height: u16,
    label: &str,
) -> (usize, usize, usize, usize) {
    use crate::markdown::LinkId;
    use ratatui::buffer::CellWidth;
    use std::collections::HashMap;
    let hover = crate::theme::app_theme().markdown.link_hover;
    app.hovered_link = None;
    app.code_select = None;
    let idle = draw(app, width, height);
    let area = app.content_area;
    let (left, right) = inner_columns(app);
    let (mut wide, mut emoji) = (0, 0);
    for y in area.y..area.bottom() {
        for x in left..right {
            let cell = &idle[(x, y)];
            assert_ne!(
                cell.fg, hover,
                "{label} idle cell {x},{y} uses the hover color"
            );
            match cell.symbol() {
                "戊" | "己" | "庚" | "辛" | "壬" | "癸" => {
                    assert_eq!(
                        destination(app, x, y),
                        Some("https://wide.test"),
                        "{label} at {x},{y}"
                    );
                    wide += 1;
                }
                "👩‍💻" => {
                    assert_eq!(destination(app, x, y), None, "{label} at {x},{y}");
                    emoji += 1;
                }
                _ => {}
            }
        }
    }
    let visible = app.scroll()..(app.scroll() + usize::from(area.height)).min(app.total());
    let mut owners: Vec<((usize, usize), LinkId)> = Vec::new();
    for line in visible {
        for (index, span) in app
            .link_spans_by_line
            .get(&line)
            .into_iter()
            .flatten()
            .enumerate()
        {
            if !owners.iter().any(|(_, id)| *id == span.link_id) {
                owners.push(((line, index), span.link_id));
            }
        }
    }
    let mut expected: HashMap<(u16, u16), LinkId> = HashMap::new();
    for (hovered, id) in owners {
        app.hovered_link = Some(hovered);
        let buffer = draw(app, width, height);
        for y in area.y..area.bottom() {
            let mut x = left;
            while x < right {
                let cell = &buffer[(x, y)];
                let step = cell.symbol().cell_width().max(1);
                if cell.fg == hover {
                    for dx in 0..step.min(right - x) {
                        assert!(
                            expected.insert((x + dx, y), id).is_none(),
                            "{label} overlap at {x},{y}"
                        );
                    }
                }
                x += step;
            }
        }
    }
    app.hovered_link = None;
    let mut hits = 0;
    for y in 0..height {
        for x in 0..width {
            let actual = app.link_at_position(x, y).map(|span| span.link_id);
            assert_eq!(actual, expected.get(&(x, y)).copied(), "{label} at {x},{y}");
            hits += usize::from(actual.is_some());
            if !(left..right).contains(&x) || !(area.y..area.bottom()).contains(&y) {
                assert_eq!(
                    app.line_idx_at(x, y),
                    None,
                    "{label} outside content at {x},{y}"
                );
            }
        }
    }
    let rows = (area.y..area.bottom())
        .filter(|&y| app.line_idx_at(left, y).is_some())
        .count();
    (hits, rows, wide, emoji)
}

fn assert_double_click_rows_match_drawn_code_block(
    app: &mut App,
    width: u16,
    height: u16,
    label: &str,
    synced: bool,
) -> usize {
    let hover = crate::theme::app_theme().markdown.link_hover;
    app.hovered_link = None;
    app.code_select = None;
    let idle = draw(app, width, height);
    app.code_select = Some(0);
    let lit = draw(app, width, height);
    app.code_select = None;
    let area = app.content_area;
    let (left, right) = inner_columns(app);
    let block = &app.code_blocks[0];
    let content_width = usize::from(right - left).saturating_sub(app.line_number_gutter_width());
    let exact = synced && block.rendered_offset + block.rendered_width <= content_width;
    let offset = block.rendered_offset as u16;
    let mut painted_rows = 0;
    for y in area.y..area.bottom() {
        let painted =
            (area.x..area.right()).any(|x| lit[(x, y)].fg == hover && idle[(x, y)].fg != hover);
        let hit = app
            .line_idx_at(left, y)
            .and_then(|line| app.code_block_at(line, offset));
        if exact {
            assert_eq!(hit.is_some(), painted, "{label} code block row {y}");
        } else if painted {
            assert!(hit.is_some(), "{label} code block row {y}");
        }
        painted_rows += usize::from(painted);
    }
    painted_rows
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
    app.hovered_link = app.find_hovered_link(x, y);
    let hovered = draw(&mut app, 23, 12);
    for (x, y) in label_cells {
        assert_eq!(
            hovered[(x, y)].fg,
            crate::theme::app_theme().markdown.link_hover
        );
    }
}

#[test]
fn code_block_rows_follow_actual_heading_reflow() {
    let _guard = lock_theme_test_state();
    let (ss, _) = test_assets();
    let ts = super::test_theme_set();
    for gutter in [false, true] {
        let mut app = app_for(
            "## Download https://github.com/RivoLink/leaf/releases/v1.29.0.tar.gz\n\n```\ncode\n```\n\nafter",
            47,
        );
        if gutter {
            app.toggle_line_numbers();
            app.sync_render_width(47 - app.line_number_gutter_width(), &ss, ts);
        }
        let buffer = draw(&mut app, 50, 12);
        let inner_x = 1 + app.line_number_gutter_width() as u16;
        let drawn: Vec<u16> = (0..12)
            .filter(|&y| ["┌", "│", "└"].contains(&buffer[(inner_x, y)].symbol()))
            .collect();
        let rows: Vec<u16> = (0..12)
            .filter(|&y| {
                app.line_idx_at(inner_x + 1, y)
                    .is_some_and(|line| app.code_block_at(line, 1) == Some(0))
            })
            .collect();
        assert_eq!(drawn.len(), 3, "gutter={gutter}");
        assert_eq!(rows, drawn, "gutter={gutter}");
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
fn replacing_content_invalidates_old_screen_hit_regions() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("[A](https://a.test)", 60);
    draw(&mut app, 63, 8);
    assert_eq!(destination(&app, 2, 0), Some("https://a.test"));
    assert_eq!(app.line_idx_at(2, 0), Some(0));
    app.replace_content(parse_markdown_for_test("no links", 60));
    assert_eq!(destination(&app, 2, 0), None);
    assert_eq!(app.line_idx_at(2, 0), None);
}

#[test]
fn tab_separated_hover_targets_only_the_correct_occurrence() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("[A](https://a.test)\t[B](https://b.test)", 60);
    let idle = draw(&mut app, 63, 8);
    assert_eq!(destination(&app, 4, 0), Some("https://b.test"));
    app.hovered_link = app.find_hovered_link(4, 0);
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
fn search_highlight_keeps_grapheme_links_clickable() {
    let _guard = lock_theme_test_state();
    use ratatui::buffer::CellWidth;

    let mut app = app_for("[👩‍💻](https://emoji.test) [After](https://after.test)", 60);
    draw(&mut app, 63, 8);
    app.set_search_query("👩");
    app.run_search();
    let buffer = draw(&mut app, 63, 8);
    let area = app.content_area;
    let (left, right) = inner_columns(&app);
    let mut emoji_cells = 0;
    let mut after_cells = 0;

    for y in area.y..area.bottom() {
        for x in left..right {
            match buffer[(x, y)].symbol() {
                "👩‍💻" => {
                    let width = buffer[(x, y)].symbol().cell_width().max(1);
                    for dx in 0..width {
                        assert_eq!(
                            destination(&app, x + dx, y),
                            Some("https://emoji.test"),
                            "emoji cell {},{y}",
                            x + dx
                        );
                    }
                    emoji_cells += usize::from(width);
                }
                "A" | "f" | "t" | "e" | "r" => {
                    assert_eq!(
                        destination(&app, x, y),
                        Some("https://after.test"),
                        "After cell {x},{y}"
                    );
                    after_cells += 1;
                }
                _ => {}
            }
        }
    }

    assert_eq!(emoji_cells, 2);
    assert_eq!(after_cells, 5);
}

#[test]
fn empty_or_clipped_viewport_does_not_create_offscreen_hits() {
    let _guard = lock_theme_test_state();
    let mut app = app_for(
        "# many words many words many words [Late](https://late.test)",
        60,
    );
    draw(&mut app, 15, 3);
    assert!(
        app.screen_geometry.links.is_empty(),
        "late heading link is below the actual viewport"
    );
    assert!(app.find_hovered_link(3, 1).is_none());
    for (width, height) in [(3u16, 6u16), (63, 1)] {
        let mut app = app_for("[A](https://a.test) [B](https://b.test)", 60);
        draw(&mut app, width, height);
        for y in 0..height {
            for x in 0..width {
                assert!(
                    app.link_at_position(x, y).is_none(),
                    "size={width}x{height} at {x},{y}"
                );
                assert_eq!(
                    app.line_idx_at(x, y),
                    None,
                    "size={width}x{height} at {x},{y}"
                );
            }
        }
    }
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

#[test]
fn clicks_match_drawn_links_across_scroll_gutter_and_toc() {
    let _guard = lock_theme_test_state();
    let mut configs = 0;
    let mut configs_with_hits = 0;
    let mut code_rows = 0;
    let (mut wide, mut emoji) = (0, 0);
    for width in [44u16, 90] {
        for gutter in [false, true] {
            for toc in [false, true] {
                let mut app = app_for(DOC, 60);
                if gutter {
                    app.toggle_line_numbers();
                }
                if toc {
                    app.toggle_toc();
                }
                sync_width(&mut app, width, 9);
                for height in [9u16, 16] {
                    app.scroll_top();
                    draw(&mut app, width, height);
                    let max = app.max_scroll();
                    let mut scrolls = vec![0, 1, max / 2, max];
                    scrolls.sort_unstable();
                    scrolls.dedup();
                    for scroll in scrolls {
                        app.scroll_top();
                        app.scroll_down(scroll);
                        let label = format!(
                            "w={width} h={height} gutter={gutter} toc={toc} scroll={}",
                            app.scroll()
                        );
                        let (hits, rows, wide_cells, emoji_cells) =
                            assert_clicks_match_drawn_links(&mut app, width, height, &label);
                        assert_eq!(
                            rows,
                            usize::from(app.content_area.height),
                            "{label} every content row maps to a line"
                        );
                        code_rows += assert_double_click_rows_match_drawn_code_block(
                            &mut app, width, height, &label, true,
                        );
                        configs += 1;
                        configs_with_hits += usize::from(hits > 0);
                        wide += wide_cells;
                        emoji += emoji_cells;
                    }
                }
            }
        }
    }
    assert!(configs_with_hits * 10 >= configs * 9);
    assert!(code_rows > 0);
    assert!(wide > 0 && emoji > 0);
}

#[test]
fn seeded_geometry_matches_paragraph_rows_and_cells() {
    let _guard = lock_theme_test_state();
    use crate::markdown::{width::rendered_span_width, LinkId, LinkSpan};
    use ratatui::{
        buffer::CellWidth,
        layout::Rect,
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Paragraph, Wrap},
    };
    use std::collections::{BTreeSet, HashMap};
    const PIECES: &[&str] = &[
        "a",
        "bc",
        "word",
        "x",
        " ",
        "  ",
        "\t",
        "界",
        "한글",
        "é",
        "e\u{301}",
        "👩‍💻",
        "\u{a0}",
        "\u{200b}",
        "ｶﾞ",
        "abcdefghij",
        "-",
        "\u{7}",
        "   ",
    ];
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = |n: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n as u64) as usize
    };
    let mut generated_spans = 0usize;
    for case in 0..3000 {
        let width = 1 + next(18);
        let height = 1 + next(10);
        let first_line = next(4);
        let gutter = if next(2) == 0 { 0 } else { 3 };
        let mut lines = Vec::new();
        let mut ranges: HashMap<usize, Vec<LinkSpan>> = HashMap::new();
        let mut link_colors: HashMap<(usize, usize), u8> = HashMap::new();
        let mut next_link = 1u8;
        for offset in 0..1 + next(5) {
            let line_idx = first_line + offset;
            let bg = Color::Indexed(200 + offset as u8);
            let mut spans = Vec::new();
            if gutter > 0 {
                spans.push(Span::styled("1│ ", Style::default().bg(bg)));
            }
            let mut col = 0;
            for _ in 0..next(6) {
                let mut text = String::new();
                for _ in 0..1 + next(4) {
                    text.push_str(PIECES[next(PIECES.len())]);
                }
                generated_spans += 1;
                if generated_spans.is_multiple_of(5) {
                    text.push_str("\r\n");
                }
                let link = next(3) == 0;
                let fg = if link {
                    Color::Indexed(next_link)
                } else {
                    Color::Indexed(0)
                };
                let span = Span::styled(text, Style::default().fg(fg).bg(bg));
                let span_width = rendered_span_width(&span);
                if link && span_width > 0 {
                    let list = ranges.entry(line_idx).or_default();
                    link_colors.insert((line_idx, list.len()), next_link);
                    list.push(LinkSpan {
                        link_id: LinkId(next_link as usize),
                        line_idx,
                        start_col: col,
                        end_col: col + span_width,
                    });
                    next_link += 1;
                }
                col += span_width;
                if link && span_width == 0 {
                    spans.push(Span::styled(
                        span.content.clone(),
                        Style::default().fg(Color::Indexed(0)).bg(bg),
                    ));
                } else {
                    spans.push(span);
                }
            }
            lines.push(Line::from(spans));
        }
        let mut terminal =
            Terminal::new(TestBackend::new(width as u16 + 4, height as u16 + 1)).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(
                    Paragraph::new(lines.clone()).wrap(Wrap { trim: false }),
                    Rect::new(1, 0, width as u16, height as u16),
                )
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let geometry = crate::render::screen_geometry(
            &lines,
            &ranges,
            first_line,
            gutter,
            width as u16,
            height as u16,
        );
        let label = format!(
            "case={case} w={width} h={height} first={first_line} gutter={gutter} lines={lines:?}"
        );
        assert!(geometry.row_lines.len() <= height, "{label}");
        assert!(
            geometry
                .links
                .iter()
                .all(|r| r.end_col <= width && r.row < height),
            "{label}"
        );
        if let Some(&first) = geometry.row_lines.first() {
            assert_eq!(first, first_line, "{label}");
        }
        assert!(
            geometry
                .row_lines
                .windows(2)
                .all(|w| w[1] == w[0] || w[1] == w[0] + 1),
            "{label}"
        );
        for y in 0..height {
            let mut painted_lines = BTreeSet::new();
            let mut x = 0;
            while x < width {
                let cell = &buffer[(x as u16 + 1, y as u16)];
                if let Color::Indexed(code) = cell.bg {
                    if code >= 200 {
                        painted_lines.insert(first_line + usize::from(code - 200));
                    }
                }
                let expected = match cell.fg {
                    Color::Indexed(code) if code > 0 => Some(code),
                    _ => None,
                };
                let actual = geometry
                    .links
                    .iter()
                    .find(|r| r.row == y && r.start_col <= x && x < r.end_col)
                    .map(|r| link_colors[&(geometry.row_lines[r.row], r.span_idx)]);
                assert_eq!(actual, expected, "{label} cell {x},{y}");
                x += cell.symbol().cell_width().max(1) as usize;
            }
            match geometry.row_lines.get(y) {
                Some(&line) => assert!(
                    painted_lines.is_empty() || painted_lines == BTreeSet::from([line]),
                    "{label} row {y}: geometry={line} painted={painted_lines:?}"
                ),
                None => assert!(
                    painted_lines.is_empty(),
                    "{label} row {y} painted beyond geometry {painted_lines:?}"
                ),
            }
        }
    }
}

#[test]
fn clicks_stay_correct_across_layout_transitions() {
    let _guard = lock_theme_test_state();
    let mut app = app_for(DOC, 60);
    let mut size = (63u16, 9u16);
    let mut hits = 0;
    let mut check = |app: &mut App, size: (u16, u16), step: &str, synced: bool| {
        let label = format!("after {step} size={size:?} scroll={}", app.scroll());
        hits += assert_clicks_match_drawn_links(app, size.0, size.1, &label).0;
        assert_double_click_rows_match_drawn_code_block(app, size.0, size.1, &label, synced);
    };
    sync_width(&mut app, size.0, size.1);
    check(&mut app, size, "start", true);
    app.scroll_down(2);
    check(&mut app, size, "scroll", true);
    size.1 = 18;
    check(&mut app, size, "taller", true);
    app.toggle_line_numbers();
    check(&mut app, size, "gutter before reparse", false);
    sync_width(&mut app, size.0, size.1);
    check(&mut app, size, "gutter", true);
    app.toggle_toc();
    check(&mut app, size, "toc before reparse", false);
    sync_width(&mut app, size.0, size.1);
    check(&mut app, size, "toc", true);
    size.0 = 50;
    check(&mut app, size, "narrower before reparse", false);
    sync_width(&mut app, size.0, size.1);
    check(&mut app, size, "narrower", true);
    app.scroll_down(5);
    check(&mut app, size, "scroll narrow", true);
    app.set_max_width(Some(30));
    app.toggle_toc();
    sync_width(&mut app, size.0, size.1);
    check(&mut app, size, "toc off with max width", true);
    app.toggle_line_numbers();
    sync_width(&mut app, size.0, size.1);
    check(&mut app, size, "gutter off", true);
    assert!(hits > 0);
}

#[test]
fn screen_geometry_coalesces_adjacent_cells_for_one_link() {
    use crate::markdown::{LinkId, LinkSpan};
    use ratatui::text::Line;
    use std::collections::HashMap;

    let line = Line::from("abcdef");
    let spans = HashMap::from([(
        0,
        vec![LinkSpan {
            line_idx: 0,
            start_col: 1,
            end_col: 5,
            link_id: LinkId(0),
        }],
    )]);
    let geometry = crate::render::screen_geometry(&[line], &spans, 0, 0, 6, 1);

    assert_eq!(geometry.links.len(), 1);
    assert_eq!(
        (
            geometry.links[0].row,
            geometry.links[0].start_col,
            geometry.links[0].end_col
        ),
        (0, 1, 5)
    );
}
