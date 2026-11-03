use super::common::app_helpers::{
    draw_app as draw, markdown_app, parse_markdown_for_test, status_text,
    sync_app_width as sync_width,
};
use super::{lock_theme_test_state, test_assets};
use crate::{
    app::{App, LinkFlash},
    markdown::LinkId,
    runtime::{
        handle_link_select_key_with, handle_mouse_event_with, handle_select_mode_keys,
        SelectModeOutcome,
    },
    theme::app_theme,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    buffer::{Buffer, Cell},
    style::Modifier,
};
use std::cell::RefCell;
use unicode_segmentation::UnicodeSegmentation;

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

const DOC_PATH: &str = "/docs/selection.md";

fn app_for(source: &str, parse_width: usize) -> App {
    app_with_path(source, parse_width, Some(DOC_PATH))
}

fn app_with_path(source: &str, parse_width: usize, filepath: Option<&str>) -> App {
    markdown_app(source, parse_width, "selection.md", filepath)
}

fn content_rows(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height - 1)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

fn is_letter_cell(buffer: &Buffer, x: u16, y: u16) -> bool {
    x < buffer.area.width
        && buffer[(x, y)]
            .symbol()
            .chars()
            .next()
            .is_some_and(char::is_alphabetic)
}

fn label_cells(buffer: &Buffer, label: &str) -> Vec<(u16, u16)> {
    let mut cells = Vec::new();
    for y in 0..buffer.area.height - 1 {
        for x in 0..buffer.area.width {
            if x > 0 && is_letter_cell(buffer, x - 1, y) {
                continue;
            }
            let mut col = x;
            let mut matched = Vec::new();
            for symbol in label.graphemes(true) {
                let width = ratatui::text::Span::raw(symbol).width().max(1) as u16;
                if col + width > buffer.area.width || buffer[(col, y)].symbol() != symbol {
                    matched.clear();
                    break;
                }
                matched.push((col, y));
                col += width;
            }
            if !is_letter_cell(buffer, col, y) {
                cells.extend(matched);
            }
        }
    }
    assert!(
        !cells.is_empty(),
        "label {label:?} missing: {:?}",
        content_rows(buffer)
    );
    cells
}

fn is_selected(cell: &Cell) -> bool {
    let theme = app_theme();
    cell.fg == theme.markdown.link_text
        && cell.bg == theme.ui.content_bg
        && cell.modifier.contains(Modifier::REVERSED)
}

fn selected_cells(buffer: &Buffer) -> Vec<(u16, u16)> {
    (0..buffer.area.height - 1)
        .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
        .filter(|&position| is_selected(&buffer[position]))
        .collect()
}

fn assert_label_selected(buffer: &Buffer, label: &str) {
    for position in label_cells(buffer, label) {
        assert!(
            is_selected(&buffer[position]),
            "label {label:?} at {position:?}"
        );
    }
}

#[test]
fn selection_follows_drawn_order_and_never_scrolls() {
    let _guard = lock_theme_test_state();
    let source = "| Left | Right |\n|:---:|---:|\n| [A A A A A A A A A A A A A A A A A A A A](https://a.test) [C](https://c.test) | [B](https://b.test) |";
    let mut app = app_for(source, 39);
    assert_eq!(app.link_url(LinkId(0)), Some("https://a.test"));
    assert_eq!(app.link_url(LinkId(1)), Some("https://c.test"));
    assert_eq!(app.link_url(LinkId(2)), Some("https://b.test"));
    let idle = draw(&mut app, 42, 16);
    assert!(label_cells(&idle, "B")[0].1 < label_cells(&idle, "C")[0].1);
    let scroll = app.scroll();
    app.enter_link_select_mode();
    for (step, label, destination) in [
        (0, "A", "https://a.test"),
        (1, "B", "https://b.test"),
        (1, "C", "https://c.test"),
        (1, "A", "https://a.test"),
        (-1, "C", "https://c.test"),
    ] {
        match step {
            1 => app.link_select_next(),
            -1 => app.link_select_prev(),
            _ => {}
        }
        let selected = draw(&mut app, 42, 16);
        assert_eq!(app.selected_link_url(), Some(destination));
        assert_label_selected(&selected, label);
        assert_eq!(app.scroll(), scroll);
        assert_eq!(content_rows(&selected), content_rows(&idle));
    }
}

#[test]
fn same_url_occurrences_are_separate_stops() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("[one](https://same.test) [two](https://same.test)", 57);
    let idle = draw(&mut app, 60, 6);
    app.enter_link_select_mode();
    assert_eq!(app.selected_link(), Some(LinkId(0)));
    app.link_select_next();
    let second = draw(&mut app, 60, 6);
    assert_eq!(app.selected_link(), Some(LinkId(1)));
    assert_label_selected(&second, "two");
    for position in label_cells(&second, "one") {
        assert!(!is_selected(&second[position]));
    }
    let &(x, y) = label_cells(&second, "one").last().unwrap();
    assert_eq!(second[(x + 1, y)].symbol(), " ");
    assert!(!is_selected(&second[(x + 1, y)]));
    app.link_select_next();
    let first = draw(&mut app, 60, 6);
    assert_eq!(app.selected_link(), Some(LinkId(0)));
    assert_label_selected(&first, "one");
    assert_eq!(content_rows(&first), content_rows(&idle));
}

#[test]
fn wrapped_link_is_one_stop_even_when_partly_scrolled() {
    let _guard = lock_theme_test_state();
    let words = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron";
    for heading in [true, false] {
        let prefix = if heading { "# " } else { "" };
        let source = format!(
            "{prefix}before [{words}](https://wrapped.test) after\n\n[Follow](https://follow.test)"
        );
        let mut app = app_for(&source, 39);
        let link_lines = app
            .link_spans_by_line
            .values()
            .filter(|spans| spans.iter().any(|span| span.link_id == LinkId(0)))
            .count();
        assert_eq!(link_lines == 1, heading);
        let idle = draw(&mut app, 42, 14);
        assert!(label_cells(&idle, "alpha")[0].1 < label_cells(&idle, "omicron")[0].1);
        app.enter_link_select_mode();
        let selected = draw(&mut app, 42, 14);
        for word in words.split_whitespace() {
            assert_label_selected(&selected, word);
        }
        for word in ["before", "after", "Follow"] {
            assert!(label_cells(&selected, word)
                .iter()
                .all(|&position| !is_selected(&selected[position])));
        }
        assert_eq!(content_rows(&idle), content_rows(&selected));
        app.link_select_next();
        let next = draw(&mut app, 42, 14);
        assert_eq!(app.selected_link_url(), Some("https://follow.test"));
        assert_label_selected(&next, "Follow");
        app.link_select_next();
        assert_eq!(app.selected_link_url(), Some("https://wrapped.test"));
        assert_label_selected(&draw(&mut app, 42, 14), "omicron");
        if heading {
            continue;
        }
        app.exit_link_select_mode();
        draw(&mut app, 42, 3);
        app.scroll_down(1);
        let scrolled = draw(&mut app, 42, 3);
        assert_eq!(app.scroll(), 1);
        let rows = content_rows(&scrolled);
        assert!(!rows.iter().any(|row| row.contains("alpha")));
        assert!(!rows.iter().any(|row| row.contains("Follow")));
        app.enter_link_select_mode();
        let partial = draw(&mut app, 42, 3);
        assert_eq!(app.selected_link_url(), Some("https://wrapped.test"));
        assert_label_selected(&partial, "omicron");
        assert_eq!(content_rows(&partial), rows);
        let unchanged = selected_cells(&partial);
        for step in [true, false] {
            if step {
                app.link_select_next();
            } else {
                app.link_select_prev();
            }
            assert_eq!(app.selected_link_url(), Some("https://wrapped.test"));
            assert_eq!(selected_cells(&draw(&mut app, 42, 3)), unchanged);
        }
    }
}

#[test]
fn undrawn_links_are_not_selectable() {
    let _guard = lock_theme_test_state();
    for source in [
        format!("{}[Hidden](https://hidden.test)", "padding\n\n".repeat(20)),
        format!("# {}[Hidden](https://hidden.test)", "padding ".repeat(40)),
    ] {
        let mut app = app_for(&source, 39);
        let idle = draw(&mut app, 42, 3);
        assert!(!content_rows(&idle).iter().any(|row| row.contains("Hidden")));
        assert!(!app.link_spans_by_line.is_empty());
        if source.starts_with('#') {
            assert!(app.link_spans_by_line.contains_key(&0));
        }
        let scroll = app.scroll();
        app.enter_link_select_mode();
        let selected = draw(&mut app, 42, 3);
        assert!(!app.is_link_select_mode());
        assert_eq!(
            app.link_flash().map(|(flash, _)| flash),
            Some(&LinkFlash::NoneVisible)
        );
        assert!(selected_cells(&selected).is_empty());
        assert_eq!(app.scroll(), scroll);
        assert!(status_text(&selected).contains(" No link in view "));
    }
}

#[test]
fn selection_is_style_only_over_search_and_hover() {
    let _guard = lock_theme_test_state();
    for word in ["match", "漢字"] {
        let label = format!("{word} focus");
        let source = format!("{word} plain [{label}](https://focus.test) tail {word}");
        let mut app = app_for(&source, 87);
        app.begin_search();
        app.set_search_draft(word);
        app.confirm_search();
        let buffer = draw(&mut app, 90, 6);
        let (x, y) = label_cells(&buffer, "focus")[0];
        app.hovered_link = app.find_hovered_link(x, y);
        let idle = draw(&mut app, 90, 6);
        app.enter_link_select_mode();
        let selected = draw(&mut app, 90, 6);
        assert!(status_text(&selected).contains("https://focus.test"));
        assert!(!status_text(&idle).contains("https://focus.test"));
        assert_eq!(content_rows(&idle), content_rows(&selected));
        let link = label_cells(&idle, &format!("#{label}"));
        let matches = label_cells(&idle, word);
        assert!(matches.iter().any(|position| link.contains(position)));
        assert!(matches.iter().any(|position| !link.contains(position)));
        let theme = app_theme();
        for y in 0..5 {
            for x in 0..90 {
                let mut expected = idle[(x, y)].clone();
                if link.contains(&(x, y)) {
                    expected.fg = theme.markdown.link_text;
                    expected.bg = theme.ui.content_bg;
                    expected.modifier.insert(Modifier::REVERSED);
                    assert_ne!(selected[(x, y)], idle[(x, y)]);
                }
                assert_eq!(selected[(x, y)], expected, "{word} cell {x},{y}");
            }
        }
        for position in matches.iter().filter(|position| link.contains(position)) {
            assert!(selected[*position].modifier.contains(Modifier::BOLD));
        }
        assert!(app.exit_link_select_mode());
        let exited = draw(&mut app, 90, 6);
        assert!(!status_text(&exited).contains("https://focus.test"));
        for y in 0..5 {
            for x in 0..90 {
                assert_eq!(exited[(x, y)], idle[(x, y)]);
            }
        }
    }
}

#[test]
fn label_oracle_with_gutter_toc_table_and_footnote() {
    let _guard = lock_theme_test_state();
    let source = "# Heading\n\n| Left | Right |\n| --- | --- |\n| [Table](https://table.test) tail | plain |\n\nRef[^a].\n\n[^a]: [note](https://note.test) adjacent\n";
    for gutter in [false, true] {
        for toc in [false, true] {
            let mut app = app_for(source, 87);
            if gutter {
                app.toggle_line_numbers();
            }
            if toc {
                app.toggle_toc();
            }
            sync_width(&mut app, 90);
            let idle = draw(&mut app, 90, 20);
            assert!(content_rows(&idle).iter().any(|row| row.contains('│')));
            assert!(!app.is_link_select_mode());
            for (stop, label) in ["Table", "note"].into_iter().enumerate() {
                app.hovered_link = None;
                app.enter_link_select_mode();
                for _ in 0..stop {
                    app.link_select_next();
                }
                let selected = draw(&mut app, 90, 20);
                let destination = if label == "Table" {
                    "https://table.test"
                } else {
                    "https://note.test"
                };
                assert!(status_text(&selected).contains(destination));
                let expected = selected_cells(&selected);
                assert_label_selected(&selected, label);
                for text in ["tail", "plain", "adjacent", "Ref"] {
                    for position in label_cells(&selected, text) {
                        assert!(
                            !is_selected(&selected[position]),
                            "gutter={gutter} toc={toc} label={label} text={text}"
                        );
                    }
                }
                for y in 0..19 {
                    for x in 0..90 {
                        if selected[(x, y)].symbol() == "│"
                            || selected[(x, y)]
                                .symbol()
                                .chars()
                                .any(|ch| ch.is_ascii_digit())
                        {
                            assert!(!is_selected(&selected[(x, y)]));
                        }
                    }
                }
                assert_eq!(content_rows(&selected), content_rows(&idle));
                app.exit_link_select_mode();
                let (x, y) = expected[0];
                app.hovered_link = app.find_hovered_link(x, y);
                let hovered = draw(&mut app, 90, 20);
                let hover = app_theme().markdown.link_hover;
                let painted: Vec<_> = (0..19)
                    .flat_map(|y| (0..90).map(move |x| (x, y)))
                    .filter(|&position| hovered[position].fg == hover && idle[position].fg != hover)
                    .collect();
                assert_eq!(painted, expected, "gutter={gutter} toc={toc} label={label}");
            }
        }
    }
}

#[test]
fn hover_and_selection_keep_distinct_styles() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("[Alpha](https://a.test) [Bravo](https://b.test)", 57);
    let idle = draw(&mut app, 60, 6);
    let (x, y) = label_cells(&idle, "Bravo")[0];
    app.hovered_link = app.find_hovered_link(x, y);
    app.enter_link_select_mode();
    let selected = draw(&mut app, 60, 6);
    assert_label_selected(&selected, "Alpha");
    for position in label_cells(&selected, "Bravo") {
        assert_eq!(selected[position].fg, app_theme().markdown.link_hover);
        assert!(!is_selected(&selected[position]));
    }
    let (x, y) = label_cells(&idle, "Alpha")[0];
    app.hovered_link = app.find_hovered_link(x, y);
    assert_label_selected(&draw(&mut app, 60, 6), "Alpha");
}

#[test]
fn content_replacement_drops_selection() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("[Old](https://old.test)", 57);
    draw(&mut app, 60, 6);
    app.enter_link_select_mode();
    assert_eq!(app.selected_link(), Some(LinkId(0)));
    let selected = draw(&mut app, 60, 6);
    assert!(status_text(&selected).contains("https://old.test"));
    assert_label_selected(&selected, "Old");
    app.replace_content(parse_markdown_for_test("[New](https://new.test)", 57));
    assert!(!app.is_link_select_mode());
    let replaced = draw(&mut app, 60, 6);
    assert!(!status_text(&replaced).contains("https://"));
    assert!(selected_cells(&replaced).is_empty());
    app.enter_link_select_mode();
    assert_eq!(app.selected_link(), Some(LinkId(0)));
    assert_eq!(app.selected_link_url(), Some("https://new.test"));
    let new_selection = draw(&mut app, 60, 6);
    assert_label_selected(&new_selection, "New");
    assert!(status_text(&new_selection).contains("https://new.test"));
}

#[test]
fn selection_dropped_when_link_leaves_drawn_frame() {
    let _guard = lock_theme_test_state();
    let source = format!("[Top](https://top.test)\n\n{}", "padding\n\n".repeat(30));
    let mut app = app_for(&source, 57);
    draw(&mut app, 60, 6);
    app.enter_link_select_mode();
    let selected = draw(&mut app, 60, 6);
    assert_label_selected(&selected, "Top");
    assert!(status_text(&selected).contains("https://top.test"));
    app.scroll_down(10);
    let away = draw(&mut app, 60, 6);
    assert!(!status_text(&away).contains("https://top.test"));
    assert!(!app.is_link_select_mode());
    assert!(selected_cells(&away).is_empty());
    app.scroll_top();
    let back = draw(&mut app, 60, 6);
    assert!(!status_text(&back).contains("https://top.test"));
    assert!(selected_cells(&back).is_empty());
    assert!(!app.is_link_select_mode());
}

#[test]
fn entering_selection_clears_goto_target() {
    let _guard = lock_theme_test_state();
    for has_link in [true, false] {
        let source = if has_link {
            "[Target](https://target.test) tail"
        } else {
            "plain target row"
        };
        let mut app = app_for(source, 57);
        app.begin_goto_line();
        app.push_goto_draft('1');
        app.confirm_goto_line();
        let target = app.goto_line_target().unwrap();
        let idle = draw(&mut app, 60, 7);
        let goto_bg = app_theme().markdown.search_highlight_bg;
        let label = if has_link { "Target" } else { "plain" };
        let row = label_cells(&idle, label)[0].1;
        assert!((0..60).any(|x| idle[(x, row)].bg == goto_bg));
        app.enter_link_select_mode();
        let selected = draw(&mut app, 60, 7);
        if has_link {
            assert_eq!(app.goto_line_target(), None);
            assert_label_selected(&selected, "Target");
            assert!((0..60).all(|x| selected[(x, row)].bg != goto_bg));
            app.exit_link_select_mode();
            let exited = draw(&mut app, 60, 7);
            assert!((0..60).all(|x| exited[(x, row)].bg != goto_bg));
        } else {
            assert_eq!(app.goto_line_target(), Some(target));
            assert!(status_text(&selected).contains(" No link in view "));
            assert!(selected_cells(&selected).is_empty());
        }
    }
}

#[test]
fn status_shows_destination_updates_and_clears() {
    let _guard = lock_theme_test_state();
    let mut app = app_for(
        "[First](https://first.test) [Second](https://second.test)",
        117,
    );
    let idle = draw(&mut app, 120, 7);
    assert!(!status_text(&idle).contains("https://"));
    app.enter_link_select_mode();
    let first = draw(&mut app, 120, 7);
    assert!(status_text(&first).contains("https://first.test"));
    assert!(status_text(&first).contains("x/X next/prev"));
    for hint in ["enter copy", "ctrl+o open", "esc cancel"] {
        assert!(status_text(&first).contains(hint));
    }
    app.link_select_next();
    let second = draw(&mut app, 120, 7);
    assert!(status_text(&second).contains("https://second.test"));
    assert!(!status_text(&second).contains("https://first.test"));
    app.set_link_flash(LinkFlash::Copied);
    let flashed = draw(&mut app, 120, 7);
    assert_eq!(status_text(&flashed), status_text(&second));
    app.exit_link_select_mode();
    assert!(!status_text(&draw(&mut app, 120, 7)).contains("https://"));
    assert!(!app.toggle_mouse_capture());
    assert!(status_text(&draw(&mut app, 120, 7)).contains("Mouse capture disabled"));
    app.enter_link_select_mode();
    let capture_off = draw(&mut app, 120, 7);
    assert!(status_text(&capture_off).contains("https://first.test"));
    assert!(!status_text(&capture_off).contains("Mouse capture disabled"));
    assert_eq!(
        handle_select_mode_keys(&mut app, &key(KeyCode::Esc, KeyModifiers::NONE)),
        SelectModeOutcome::Consumed
    );
    assert!(status_text(&draw(&mut app, 120, 7)).contains("Mouse capture disabled"));
    let mut empty = app_for("plain text", 117);
    empty.toggle_mouse_capture();
    draw(&mut empty, 120, 7);
    empty.enter_link_select_mode();
    let no_link = draw(&mut empty, 120, 7);
    assert_eq!(
        empty.link_flash().map(|(flash, _)| flash),
        Some(&LinkFlash::NoneVisible)
    );
    assert!(status_text(&no_link).contains("Mouse capture disabled"));
    assert!(!status_text(&no_link).contains("No link in view"));
}

#[test]
fn status_line_fits_width() {
    let _guard = lock_theme_test_state();
    for destination in [
        format!("https://a.test/{}", "a".repeat(100)),
        format!("https://a.test/{}", "漢字".repeat(50)),
    ] {
        let mut app = app_for(&format!("[Label]({destination})"), 117);
        draw(&mut app, 42, 7);
        app.enter_link_select_mode();
        for width in [42, 43, 79, 80, 120] {
            draw(&mut app, width, 7);
            let text: String = app
                .status_line()
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();
            assert!(
                app.status_line().width() <= width as usize,
                "width={width} {text:?}"
            );
            assert!(text.starts_with(" leaf "));
            assert!(text.contains("https://a.test/"));
            assert!(text.contains('…'));
            assert!(!text.contains("next/prev"));
            assert_eq!(app.selected_link_url(), Some(destination.as_str()));
        }
    }
    let mut app = app_for("[Label](https://short.test)", 117);
    draw(&mut app, 120, 7);
    app.enter_link_select_mode();
    let selection = app.selected_link();
    let percent = app.scroll_percent();
    for width in [81, 80, 79, 80] {
        let buffer = draw(&mut app, width, 7);
        assert_eq!(app.scroll_percent(), percent);
        assert_eq!(app.selected_link(), selection);
        assert_label_selected(&buffer, "Label");
        assert!(app.status_line().width() <= width as usize, "width={width}");
        assert!(status_text(&buffer).contains("https://short.test"));
        assert_eq!(
            status_text(&buffer).contains("x/X next/prev · enter copy · ctrl+o open · esc cancel"),
            width >= 80,
            "width={width}"
        );
    }
}

#[test]
fn copy_keys_copy_selected_link_once() {
    let _guard = lock_theme_test_state();
    let enter = key(KeyCode::Enter, KeyModifiers::NONE);
    let ctrl_x = key(KeyCode::Char('x'), KeyModifiers::CONTROL);
    for (destination, event, succeeds) in [
        ("https://example.test/한글?a=1&b=2", enter, true),
        ("https://example.test/한글?a=1&b=2", ctrl_x, true),
        ("https://example.test/한글?a=1&b=2", ctrl_x, false),
        ("https://example.test/한글?a=1&b=2", enter, false),
        ("#top", ctrl_x, true),
    ] {
        let source = format!("[First](https://first.test) [Selected]({destination})");
        let mut app = app_for(&source, 117);
        draw(&mut app, 120, 7);
        app.enter_link_select_mode();
        app.link_select_next();
        let selected = draw(&mut app, 120, 7);
        assert_eq!(app.selected_link(), Some(LinkId(1)));
        assert_label_selected(&selected, "Selected");
        assert!(status_text(&selected).contains(destination));
        let scroll = app.scroll();
        let copied = RefCell::new(Vec::new());
        let opened = RefCell::new(Vec::new());
        assert!(handle_link_select_key_with(
            &mut app,
            &event,
            |text| {
                copied.borrow_mut().push(text.to_owned());
                succeeds
            },
            |url| opened.borrow_mut().push(url.to_owned()),
        ));
        let expected = if destination.starts_with('#') {
            "selection.md"
        } else {
            destination
        };
        assert_eq!(*copied.borrow(), [expected]);
        assert!(opened.borrow().is_empty());
        assert!(!app.is_link_select_mode());
        assert_eq!(app.scroll(), scroll);
        let expected_flash = if succeeds {
            LinkFlash::Copied
        } else {
            LinkFlash::CopyFailed
        };
        assert_eq!(
            app.link_flash().map(|(flash, _)| flash),
            Some(&expected_flash)
        );
        let exited = draw(&mut app, 120, 7);
        let message = if succeeds {
            "Copied to clipboard"
        } else {
            "Copy failed:"
        };
        assert!(status_text(&exited).contains(message));
        assert!(selected_cells(&exited).is_empty());
    }
}

#[test]
fn ctrl_o_opens_like_ctrl_click_and_keeps_selection() {
    let _guard = lock_theme_test_state();
    for (destination, filepath, expected) in [
        ("https://a.test", Some(DOC_PATH), Some("https://a.test")),
        ("mailto:x@y.test", Some(DOC_PATH), Some("mailto:x@y.test")),
        ("docs/a.pdf", Some(DOC_PATH), Some("docs/a.pdf")),
        ("#x", Some(DOC_PATH), Some(DOC_PATH)),
        ("#x", None, None),
    ] {
        let source = format!("[First](https://first.test) [Selected]({destination})");
        let mut app = app_with_path(&source, 117, filepath);
        draw(&mut app, 120, 7);
        app.enter_link_select_mode();
        app.link_select_next();
        let selected = draw(&mut app, 120, 7);
        assert_eq!(app.selected_link(), Some(LinkId(1)));
        assert_label_selected(&selected, "Selected");
        assert!(status_text(&selected).contains(destination));
        let copied = RefCell::new(Vec::new());
        let opened = RefCell::new(Vec::new());
        assert!(handle_link_select_key_with(
            &mut app,
            &key(KeyCode::Char('o'), KeyModifiers::CONTROL),
            |text| {
                copied.borrow_mut().push(text.to_owned());
                true
            },
            |url| opened.borrow_mut().push(url.to_owned()),
        ));
        assert!(copied.borrow().is_empty());
        let expected: Vec<_> = expected.into_iter().collect();
        assert_eq!(*opened.borrow(), expected, "{destination} {filepath:?}");
        assert_eq!(app.selected_link(), Some(LinkId(1)));
        assert!(app.link_flash().is_none());
        let after = draw(&mut app, 120, 7);
        assert_eq!(after, selected);
    }
}

#[test]
fn mouse_copy_exits_selection_and_ctrl_click_keeps_it() {
    let _guard = lock_theme_test_state();
    let source = "[First](https://first.test) [Second](https://second.test) [Top](#top)";
    for (label, modifiers, clicks, copy, open) in [
        (
            "Second",
            KeyModifiers::ALT,
            1,
            Some("https://second.test"),
            None,
        ),
        (
            "Second",
            KeyModifiers::NONE,
            2,
            Some("https://second.test"),
            None,
        ),
        (
            "Second",
            KeyModifiers::CONTROL,
            1,
            None,
            Some("https://second.test"),
        ),
        ("Top", KeyModifiers::CONTROL, 1, None, Some(DOC_PATH)),
    ] {
        let mut app = app_for(source, 117);
        let idle = draw(&mut app, 120, 7);
        let (column, row) = label_cells(&idle, label)[0];
        app.enter_link_select_mode();
        assert_label_selected(&draw(&mut app, 120, 7), "First");
        let copied = RefCell::new(Vec::new());
        let opened = RefCell::new(Vec::new());
        for _ in 0..clicks {
            handle_mouse_event_with(
                &mut app,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column,
                    row,
                    modifiers,
                },
                |text| {
                    copied.borrow_mut().push(text.to_owned());
                    true
                },
                |url| opened.borrow_mut().push(url.to_owned()),
            );
        }
        let copy: Vec<_> = copy.into_iter().collect();
        let open: Vec<_> = open.into_iter().collect();
        assert_eq!(*copied.borrow(), copy, "{label} {modifiers:?}");
        assert_eq!(*opened.borrow(), open, "{label} {modifiers:?}");
        let after = draw(&mut app, 120, 7);
        if copy.is_empty() {
            assert_eq!(app.selected_link(), Some(LinkId(0)));
            assert!(app.link_flash().is_none());
            assert_label_selected(&after, "First");
            assert!(status_text(&after).contains("https://first.test"));
        } else {
            assert!(!app.is_link_select_mode());
            assert!(selected_cells(&after).is_empty());
            assert!(status_text(&after).contains("Copied to clipboard"));
        }
    }
}

#[test]
fn effect_keys_do_nothing_outside_selection() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("[Label](https://a.test)", 57);
    draw(&mut app, 60, 7);
    for event in [
        key(KeyCode::Enter, KeyModifiers::NONE),
        key(KeyCode::Char('x'), KeyModifiers::CONTROL),
        key(KeyCode::Char('o'), KeyModifiers::CONTROL),
    ] {
        let copied = RefCell::new(Vec::new());
        let opened = RefCell::new(Vec::new());
        assert!(!handle_link_select_key_with(
            &mut app,
            &event,
            |text| {
                copied.borrow_mut().push(text.to_owned());
                true
            },
            |url| opened.borrow_mut().push(url.to_owned()),
        ));
        assert!(copied.borrow().is_empty());
        assert!(opened.borrow().is_empty());
        assert_eq!(
            handle_select_mode_keys(&mut app, &event),
            SelectModeOutcome::Inactive
        );
        assert!(!app.is_link_select_mode());
        assert!(app.link_flash().is_none());
        assert!(selected_cells(&draw(&mut app, 60, 7)).is_empty());
    }
}

const CODE_DOC: &str = "[Link](https://link.test)\n\n```\ncode line\n```\n";

fn code_frame_cells(buffer: &Buffer) -> Vec<(u16, u16)> {
    (0..buffer.area.height - 1)
        .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
        .filter(|&position| ["┌", "┐", "└", "┘", "─"].contains(&buffer[position].symbol()))
        .collect()
}

#[test]
fn x_enters_link_selection_from_idle_and_code_select() {
    let _guard = lock_theme_test_state();
    let source =
        "[First](https://first.test) [Second](https://second.test)\n\n```\ncode line\n```\n";
    let hover = app_theme().markdown.link_hover;
    for from_code in [false, true] {
        for character in ['x', 'X', 'c', 'y', 'C', 'Y'] {
            let mut app = app_for(source, 87);
            draw(&mut app, 90, 14);
            if from_code {
                app.enter_code_select_mode();
            }
            let before = draw(&mut app, 90, 14);
            let frame = code_frame_cells(&before);
            assert!(!frame.is_empty());
            assert_eq!(
                frame.iter().all(|&position| before[position].fg == hover),
                from_code
            );
            assert_eq!(
                handle_select_mode_keys(
                    &mut app,
                    &key(KeyCode::Char(character), KeyModifiers::NONE)
                ),
                SelectModeOutcome::Consumed
            );
            let after = draw(&mut app, 90, 14);
            if matches!(character, 'x' | 'X') {
                assert!(!app.is_code_select_mode());
                assert_eq!(app.selected_link(), Some(LinkId(0)));
                assert_label_selected(&after, "First");
                assert!(status_text(&after).contains("https://first.test"));
                assert!(frame.iter().all(|&position| after[position].fg != hover));
            } else {
                assert!(app.is_code_select_mode());
                assert!(!app.is_link_select_mode());
                assert!(frame.iter().all(|&position| after[position].fg == hover));
            }
        }
    }
}

#[test]
fn code_keys_and_ctrl_y_switch_from_link_select() {
    let _guard = lock_theme_test_state();
    for character in ['y', 'Y', 'c', 'C', 'x', 'X'] {
        let mut app = app_for(CODE_DOC, 87);
        draw(&mut app, 90, 14);
        app.enter_link_select_mode();
        assert_label_selected(&draw(&mut app, 90, 14), "Link");
        assert_eq!(
            handle_select_mode_keys(&mut app, &key(KeyCode::Char(character), KeyModifiers::NONE)),
            SelectModeOutcome::Consumed
        );
        let after = draw(&mut app, 90, 14);
        if matches!(character, 'x' | 'X') {
            assert!(app.is_link_select_mode());
            assert!(!app.is_code_select_mode());
            assert_label_selected(&after, "Link");
        } else {
            assert!(!app.is_link_select_mode());
            assert!(app.is_code_select_mode());
            let frame = code_frame_cells(&after);
            assert!(!frame.is_empty());
            assert!(frame
                .iter()
                .all(|&position| after[position].fg == app_theme().markdown.link_hover));
        }
    }
    let mut app = app_for("[Link](https://link.test)", 87);
    draw(&mut app, 90, 7);
    app.enter_link_select_mode();
    assert_label_selected(&draw(&mut app, 90, 7), "Link");
    assert!(app.code_blocks.is_empty());
    assert_eq!(
        handle_select_mode_keys(&mut app, &key(KeyCode::Char('y'), KeyModifiers::CONTROL)),
        SelectModeOutcome::Consumed
    );
    assert!(!app.is_link_select_mode());
    assert!(!app.is_code_select_mode());
    assert!(status_text(&draw(&mut app, 90, 7)).contains(" No code block in view "));
}

fn searched_app() -> App {
    let mut app = app_for(
        "# Heading\n\nneedle [First](https://first.test)\n\nneedle middle\n\nneedle [Last](https://last.test)",
        87,
    );
    draw(&mut app, 90, 20);
    app.begin_search();
    app.set_search_draft("needle");
    app.confirm_search();
    assert_eq!(app.search_match_count(), 3);
    app.next_match();
    assert_eq!(app.search_index(), 1);
    draw(&mut app, 90, 20);
    assert_eq!(app.max_scroll(), 0);
    app.enter_link_select_mode();
    assert_label_selected(&draw(&mut app, 90, 20), "First");
    app
}

#[test]
fn other_keys_exit_then_fall_through() {
    let _guard = lock_theme_test_state();
    for character in ['n', 'N', 'j', 't', 'l', 'q', '?', '/'] {
        let mut app = searched_app();
        assert_eq!(
            handle_select_mode_keys(&mut app, &key(KeyCode::Char(character), KeyModifiers::NONE)),
            SelectModeOutcome::Exited,
            "{character}"
        );
        assert!(!app.is_link_select_mode());
        assert!(app.has_active_search());
        assert_eq!(app.search_index(), 1);
        match character {
            'n' => {
                app.next_match();
                assert_eq!(app.search_index(), 2);
            }
            'N' => {
                app.prev_match();
                assert_eq!(app.search_index(), 0);
            }
            't' | 'l' => {
                if character == 't' {
                    app.toggle_toc();
                } else {
                    app.toggle_line_numbers();
                }
                sync_width(&mut app, 90);
                let after = draw(&mut app, 90, 20);
                assert!(!app.is_link_select_mode());
                assert!(selected_cells(&after).is_empty());
                assert_eq!(
                    handle_select_mode_keys(&mut app, &key(KeyCode::Char('x'), KeyModifiers::NONE)),
                    SelectModeOutcome::Consumed
                );
                assert_label_selected(&draw(&mut app, 90, 20), "First");
                assert_eq!(app.selected_link_url(), Some("https://first.test"));
            }
            _ => {}
        }
    }
}

#[test]
fn esc_and_ctrl_c_exit_without_clearing_search() {
    let _guard = lock_theme_test_state();
    for event in [
        key(KeyCode::Esc, KeyModifiers::NONE),
        key(KeyCode::Char('c'), KeyModifiers::CONTROL),
    ] {
        let mut app = searched_app();
        assert_eq!(
            handle_select_mode_keys(&mut app, &event),
            SelectModeOutcome::Consumed
        );
        assert!(!app.is_link_select_mode());
        assert!(app.has_active_search());
        assert_eq!(app.search_index(), 1);
        assert!(app.link_flash().is_none());
        assert!(selected_cells(&draw(&mut app, 90, 20)).is_empty());
    }
}

#[test]
fn link_selection_is_document_only() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("[Link](https://link.test)", 87);
    let (ss, theme) = test_assets();
    app.install_diff(
        crate::diff::parse_unified_diff(
            "diff --git a/a.md b/a.md\n--- a/a.md\n+++ b/a.md\n@@ -1 +1 @@\n-before\n+after\n",
        ),
        crate::app::DiffSpec::default(),
        ss,
        std::sync::Arc::new(theme),
    );
    let idle = draw(&mut app, 90, 14);
    assert_eq!(app.mode(), crate::app::AppMode::Diff);
    assert_eq!(
        handle_select_mode_keys(&mut app, &key(KeyCode::Char('x'), KeyModifiers::NONE)),
        SelectModeOutcome::Inactive
    );
    assert!(!app.is_link_select_mode());
    assert!(app.link_flash().is_none());
    let after = draw(&mut app, 90, 14);
    assert!(!status_text(&after).contains("No link in view"));
    assert_eq!(after, idle);
}

#[test]
fn mouse_wheel_exits_everywhere() {
    let _guard = lock_theme_test_state();
    let source = format!(
        "[Link](https://link.test)\n\n# Heading\n\n{}",
        "padding\n\n".repeat(20)
    );
    for kind in [MouseEventKind::ScrollUp, MouseEventKind::ScrollDown] {
        for over_toc in [false, true] {
            let mut app = app_for(&source, 87);
            app.toggle_toc();
            sync_width(&mut app, 90);
            let idle = draw(&mut app, 90, 14);
            assert_eq!(app.scroll(), 0);
            assert_eq!(label_cells(&idle, "Link")[0].1, 0);
            app.enter_link_select_mode();
            let selected = draw(&mut app, 90, 14);
            assert_label_selected(&selected, "Link");
            assert!(status_text(&selected).contains("https://link.test"));
            let area = if over_toc {
                app.toc_list_area.unwrap()
            } else {
                app.content_area
            };
            assert!(handle_mouse_event_with(
                &mut app,
                MouseEvent {
                    kind,
                    column: area.x + 1,
                    row: area.y,
                    modifiers: KeyModifiers::NONE,
                },
                |_| unreachable!(),
                |_| unreachable!(),
            ));
            assert!(!app.is_link_select_mode(), "{kind:?} over_toc={over_toc}");
            let after = draw(&mut app, 90, 14);
            assert!(
                selected_cells(&after).is_empty(),
                "{kind:?} over_toc={over_toc}"
            );
            assert!(!status_text(&after).contains("https://link.test"));
            if over_toc || kind == MouseEventKind::ScrollUp {
                assert_eq!(app.scroll(), 0);
                assert_eq!(label_cells(&after, "Link")[0].1, 0);
                assert_eq!(content_rows(&after), content_rows(&idle));
            } else {
                assert!(app.scroll() > 0);
            }
        }
    }
}
