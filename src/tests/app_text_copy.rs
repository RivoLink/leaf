use super::common::app_helpers::{
    draw_app as draw, markdown_app, parse_markdown_for_test, status_text,
};
use super::{lock_theme_test_state, test_assets, test_md_theme};
use crate::{
    app::{is_word_char, App, AppMode, CopyRange, COPY_HIGHLIGHT_DURATION_MS},
    markdown::parse_markdown_with_width,
    runtime::handle_mouse_event_with,
    theme::app_theme,
};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

const WIDTH: u16 = 60;
const HEIGHT: u16 = 16;

fn app_for(source: &str) -> App {
    markdown_app(
        source,
        usize::from(WIDTH) - 3,
        "copy.md",
        Some("/docs/copy.md"),
    )
}

fn text_cell(buffer: &Buffer, text: &str) -> (u16, u16) {
    for y in 0..buffer.area.height - 1 {
        let mut row = String::new();
        let mut cols = Vec::new();
        for x in 0..buffer.area.width {
            for _ in buffer[(x, y)].symbol().chars() {
                cols.push(x);
            }
            row.push_str(buffer[(x, y)].symbol());
        }
        if let Some(byte) = row.find(text) {
            return (cols[row[..byte].chars().count()], y);
        }
    }
    panic!("{text:?} not drawn");
}

fn double_click(
    app: &mut App,
    position: (u16, u16),
    modifiers: KeyModifiers,
) -> (Vec<String>, bool) {
    clicks(app, position, modifiers, 2, true)
}

fn clicks(
    app: &mut App,
    (column, row): (u16, u16),
    modifiers: KeyModifiers,
    count: usize,
    copy_succeeds: bool,
) -> (Vec<String>, bool) {
    let copied = RefCell::new(Vec::new());
    let mut handled = false;
    for _ in 0..count {
        handled = handle_mouse_event_with(
            app,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                modifiers,
            },
            |text| {
                copied.borrow_mut().push(text.to_owned());
                copy_succeeds
            },
            |_| {},
        );
    }
    (copied.into_inner(), handled)
}

fn copy_at(source: &str, text: &str, modifiers: KeyModifiers) -> Vec<String> {
    let mut app = app_for(source);
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    double_click(&mut app, text_cell(&buffer, text), modifiers).0
}

#[test]
fn word_chars_join_inner_apostrophes_and_hyphens_only() {
    assert!(is_word_char(None, 'é', None));
    assert!(is_word_char(None, '_', None));
    assert!(is_word_char(None, '語', None));
    assert!(is_word_char(Some('d'), '\'', Some('h')));
    assert!(is_word_char(Some('l'), '’', Some('a')));
    assert!(is_word_char(Some('e'), '-', Some('m')));
    assert!(!is_word_char(None, '\'', Some('h')));
    assert!(!is_word_char(Some('e'), '-', Some(' ')));
    assert!(!is_word_char(None, '.', None));
    assert!(!is_word_char(None, ' ', None));
}

#[test]
fn double_click_copies_and_highlights_the_word() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("Hello aujourd’hui world.");
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let (column, row) = text_cell(&buffer, "aujourd’hui");
    let (copied, handled) = double_click(&mut app, (column + 3, row), KeyModifiers::NONE);
    assert!(handled);
    assert_eq!(copied, ["aujourd’hui"]);

    let after = draw(&mut app, WIDTH, HEIGHT);
    assert!(status_text(&after).contains("Copied to clipboard"));
    let highlight = app_theme().markdown.search_highlight_bg;
    for x in column..column + 11 {
        assert_eq!(after[(x, row)].bg, highlight, "column {x}");
    }
    assert_ne!(after[(column - 1, row)].bg, highlight);
    assert_ne!(after[(column + 11, row)].bg, highlight);
    let (hello, _) = text_cell(&after, "Hello");
    assert_ne!(after[(hello, row)].bg, highlight);
}

#[test]
fn double_click_outside_a_word_copies_nothing() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("- Hello world.");
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let (world, row) = text_cell(&buffer, "world.");
    let (bullet, _) = text_cell(&buffer, "•");
    for column in [world - 1, world + 5, bullet] {
        let (copied, handled) = double_click(&mut app, (column, row), KeyModifiers::NONE);
        assert!(copied.is_empty(), "column {column}");
        assert!(!handled, "column {column}");
    }
    assert!(app.copy_highlight().is_none());
}

#[test]
fn single_click_copies_nothing() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("Hello world");
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let position = text_cell(&buffer, "world");
    let (copied, _) = clicks(&mut app, position, KeyModifiers::NONE, 1, true);
    assert!(copied.is_empty());
}

#[test]
fn ctrl_or_alt_double_click_copies_the_whole_wrapped_line() {
    let _guard = lock_theme_test_state();
    let source = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron";
    for (text, modifiers) in [
        ("alpha", KeyModifiers::CONTROL),
        ("omicron", KeyModifiers::CONTROL),
        ("omicron", KeyModifiers::ALT),
    ] {
        let mut app = app_for(source);
        let buffer = draw(&mut app, WIDTH, HEIGHT);
        let (alpha_row, omicron_row) = (
            text_cell(&buffer, "alpha").1,
            text_cell(&buffer, "omicron").1,
        );
        assert_ne!(alpha_row, omicron_row, "source must wrap");
        let (copied, _) = double_click(&mut app, text_cell(&buffer, text), modifiers);
        assert_eq!(copied, [source], "{text} {modifiers:?}");
        assert_eq!(app.copy_highlight().unwrap().0.len(), 2);
    }
}

#[test]
fn ctrl_double_click_copies_the_whole_soft_broken_paragraph() {
    let _guard = lock_theme_test_state();
    let source = "first line here\nsecond line here";
    let mut app = app_for(source);
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let (first, second) = (text_cell(&buffer, "first"), text_cell(&buffer, "second"));
    assert_ne!(
        first.1, second.1,
        "soft break must stay a visual line break"
    );
    for cell in [first, second] {
        let (copied, _) = double_click(&mut app, cell, KeyModifiers::CONTROL);
        assert_eq!(copied, ["first line here second line here"], "{cell:?}");
        assert_eq!(app.copy_highlight().unwrap().0.len(), 2);
    }
}

#[test]
fn ctrl_double_click_stops_at_hard_breaks_and_nested_blocks() {
    let _guard = lock_theme_test_state();
    for (source, text, expected) in [
        ("alpha one  \nbeta two", "beta", "beta two"),
        ("- parent one\n  parent two\n  - child", "child", "child"),
        (
            "- parent one\n  parent two\n  - child",
            "two",
            "parent one parent two",
        ),
    ] {
        assert_eq!(
            copy_at(source, text, KeyModifiers::CONTROL),
            [expected],
            "{source:?}"
        );
    }
}

#[test]
fn ctrl_double_click_joins_soft_broken_cjk_lines_without_spaces() {
    let _guard = lock_theme_test_state();
    assert_eq!(
        copy_at("abc中文\n中文", "abc", KeyModifiers::CONTROL),
        ["abc中文中文"]
    );
}

#[test]
fn ctrl_double_click_joins_wrapped_cjk_rows_without_spaces() {
    let _guard = lock_theme_test_state();
    let source = format!("abc{}", "漢字".repeat(30));
    let mut app = app_for(&source);
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let (copied, _) = double_click(&mut app, text_cell(&buffer, "abc"), KeyModifiers::CONTROL);
    assert_eq!(copied, [source]);
    assert!(
        app.copy_highlight().unwrap().0.len() > 1,
        "source must wrap"
    );
}

#[test]
fn soft_breaks_keep_one_line_number_per_source_line() {
    let parsed = parse_markdown_for_test("alpha\nbeta\ngamma", 80);
    assert_eq!(&parsed.line_number_map[..3], [1, 2, 3]);
    assert_eq!(&parsed.paragraph_map[..3], [1, 1, 1]);
}

#[test]
fn ctrl_double_click_in_table_copies_only_the_clicked_cell() {
    let _guard = lock_theme_test_state();
    let source = "| Name | Notes | Code |\n|---|---|---|\n| alpha | one two | `a│b` |";
    let ctrl = KeyModifiers::CONTROL;
    assert_eq!(copy_at(source, "alpha", ctrl), ["alpha"]);
    assert_eq!(copy_at(source, "two", ctrl), ["one two"]);
    assert_eq!(copy_at(source, "Name", ctrl), ["Name"]);
    assert_eq!(copy_at(source, "a│b", ctrl), ["a│b"]);

    let mut app = app_for(source);
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let (_, row) = text_cell(&buffer, "alpha");
    let (copied, _) = double_click(&mut app, (0, row), ctrl);
    assert!(copied.is_empty());
}

#[test]
fn ctrl_double_click_in_wrapped_table_cell_joins_its_lines() {
    let _guard = lock_theme_test_state();
    let source = "| A | B |\n|---|---|\n| x | alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma |";
    assert_eq!(
        copy_at(source, "alpha", KeyModifiers::CONTROL),
        ["alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma"]
    );
    assert_eq!(copy_at(source, "x", KeyModifiers::CONTROL), ["x"]);
}

#[test]
fn line_copy_strips_list_quote_and_task_decorations_only() {
    let _guard = lock_theme_test_state();
    for (source, text, expected) in [
        ("- one item", "one", "one item"),
        ("- top\n  - two item", "two", "two item"),
        ("- top\n  - mid\n    - three item", "three", "three item"),
        ("1. ordered item", "ordered", "ordered item"),
        ("1. top\n   1. nested item", "nested", "nested item"),
        ("1. top\n\n   10. tenth item", "tenth", "tenth item"),
        ("10) paren item", "paren", "paren item"),
        ("2024. year item", "year", "year item"),
        ("> [!NOTE]\n> body", "Note", "Note"),
        ("> [!WARNING]\n> body", "Warning", "Warning"),
        ("# 1. Heading", "Heading", "1. Heading"),
        ("1\\. escaped item", "escaped", "1. escaped item"),
        (
            "Text ref[^1] here.\n\n[^1]: note body",
            "Text",
            "Text ref here.",
        ),
        ("Text ref[^1] here.\n\n[^1]: note body", "body", "note body"),
        ("- [x] done task", "done", "done task"),
        ("- [ ] todo task", "todo", "todo task"),
        ("> quoted text", "quoted", "quoted text"),
        ("> outer\n>\n> > nested text", "nested", "nested text"),
        (
            "(voir ci-dessous) « Bonjour »",
            "voir",
            "(voir ci-dessous) « Bonjour »",
        ),
    ] {
        assert_eq!(
            copy_at(source, text, KeyModifiers::CONTROL),
            [expected],
            "{source:?}"
        );
    }
}

#[test]
fn line_number_gutter_is_never_copied() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("Hello world");
    app.toggle_line_numbers();
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let (column, row) = text_cell(&buffer, "1│");
    let (copied, _) = double_click(&mut app, (column, row), KeyModifiers::NONE);
    assert!(copied.is_empty());
    let (copied, _) = double_click(&mut app, text_cell(&buffer, "world"), KeyModifiers::NONE);
    assert_eq!(copied, ["world"]);
    let (copied, _) = double_click(&mut app, text_cell(&buffer, "world"), KeyModifiers::CONTROL);
    assert_eq!(copied, ["Hello world"]);
}

#[test]
fn code_block_copies_block_or_raw_line() {
    let _guard = lock_theme_test_state();
    let source = "```rust\n    let x = 1;\nfn main() {}\n```";
    let block = "    let x = 1;\nfn main() {}\n";
    for (text, modifiers, expected) in [
        ("let", KeyModifiers::NONE, block),
        ("let", KeyModifiers::CONTROL, "    let x = 1;"),
        ("main", KeyModifiers::ALT, "fn main() {}"),
        ("rust", KeyModifiers::CONTROL, block),
    ] {
        let mut app = app_for(source);
        let buffer = draw(&mut app, WIDTH, HEIGHT);
        let (copied, handled) = double_click(&mut app, text_cell(&buffer, text), modifiers);
        assert!(handled);
        if expected == block {
            assert!(copied.is_empty(), "{text} {modifiers:?}");
            assert_eq!(app.code_select, Some(0));
            assert!(app.copy_highlight().is_none());
        } else {
            assert_eq!(copied, [expected], "{text} {modifiers:?}");
            assert!(app.copy_highlight().is_some());
        }
    }
}

#[test]
fn code_line_copy_follows_wrapped_rows() {
    let _guard = lock_theme_test_state();
    let long = "let value = first_part + second_part + third_part + fourth_part + fifth_part;";
    let mut app = app_for(&format!("```rust\n{long}\nlet y = 2;\n```"));
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let (_, first_row) = text_cell(&buffer, "let value");
    let (column, row) = text_cell(&buffer, "fifth_part");
    assert_ne!(first_row, row, "code line must wrap");
    let (copied, _) = double_click(&mut app, (column, row), KeyModifiers::CONTROL);
    assert_eq!(copied, [long]);
    assert_eq!(app.copy_highlight().unwrap().0.len(), 2);

    let highlight = app_theme().markdown.search_highlight_bg;
    let after = draw(&mut app, WIDTH, HEIGHT);
    let (let_col, let_row) = text_cell(&after, "let value");
    assert_eq!(after[(let_col, let_row)].bg, highlight);
    let (y_col, y_row) = text_cell(&after, "let y");
    assert_ne!(after[(y_col, y_row)].bg, highlight);
}

#[test]
fn code_line_copy_in_file_mode() {
    let _guard = lock_theme_test_state();
    let (ss, theme) = test_assets();
    let (source, file_mode) = App::wrap_as_code_block("fn a() {}\nfn b() {}\n".into(), "rs", &ss);
    assert!(file_mode);
    let parsed = parse_markdown_with_width(
        &source,
        &ss,
        &theme,
        usize::from(WIDTH) - 3,
        &test_md_theme(),
        true,
        true,
    );
    let mut app = markdown_app(&source, usize::from(WIDTH) - 3, "a.rs", Some("/src/a.rs"));
    app.set_file_mode(true);
    app.replace_content(parsed);
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let (copied, _) = double_click(&mut app, text_cell(&buffer, "b()"), KeyModifiers::CONTROL);
    assert_eq!(copied, ["fn b() {}"]);
}

#[test]
fn embedded_blocks_never_split_into_lines() {
    let _guard = lock_theme_test_state();
    let source = "```rust\nlet a = 1;\n```\n\n```mermaid\ngraph TD\nA-->B\n```\n\n```tex\nx^2\n```\n\n```latex\ny^2\n```\n\n$$\nz^2\n$$";
    let app = app_for(source);
    let rows: Vec<_> = app.code_blocks.iter().map(|b| b.code_rows.len()).collect();
    assert_eq!(rows, [1, 0, 0, 0, 0]);
    for (idx, block) in app.code_blocks.iter().enumerate().skip(1) {
        assert!(app.code_line_at(idx, block.rendered_start + 1).is_none());
    }
    assert_eq!(
        app.code_line_at(0, app.code_blocks[0].rendered_start + 1)
            .map(|target| target.text),
        Some("let a = 1;".to_string())
    );
}

#[test]
fn links_keep_their_double_click_behavior() {
    let _guard = lock_theme_test_state();
    let source = "See [Docs](https://docs.test) now";
    for (modifiers, copies) in [(KeyModifiers::NONE, 1), (KeyModifiers::ALT, 2)] {
        assert_eq!(
            copy_at(source, "Docs", modifiers),
            vec!["https://docs.test"; copies],
            "{modifiers:?}"
        );
    }
}

#[test]
fn diff_mode_ignores_text_copy() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("Hello world");
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    app.set_mode(AppMode::Diff);
    let (copied, handled) = double_click(&mut app, text_cell(&buffer, "world"), KeyModifiers::NONE);
    assert!(copied.is_empty());
    assert!(!handled);
}

#[test]
fn highlight_is_cleared_by_content_and_mode_changes() {
    let _guard = lock_theme_test_state();
    let range = CopyRange {
        line_idx: 0,
        start_col: 0,
        end_col: 5,
    };
    let mut app = app_for("Hello world");
    app.set_copy_highlight(vec![range]);
    assert_eq!(app.copy_highlight().unwrap().0, [range]);
    app.clear_copy_highlight();
    assert!(app.copy_highlight().is_none());

    app.set_copy_highlight(vec![range]);
    app.replace_content(parse_markdown_for_test("Other text", 57));
    assert!(app.copy_highlight().is_none());

    app.set_copy_highlight(vec![range]);
    app.set_mode(AppMode::Document);
    assert!(app.copy_highlight().is_none());
}

#[test]
fn failed_copy_reports_without_highlight() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("Hello world");
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let position = text_cell(&buffer, "world");
    let (copied, _) = clicks(&mut app, position, KeyModifiers::NONE, 2, false);
    assert_eq!(copied, ["world"]);
    assert!(app.copy_highlight().is_none());
    let after = draw(&mut app, WIDTH, HEIGHT);
    assert!(!status_text(&after).contains("Copied to clipboard"));
}

#[test]
fn line_copy_ignores_the_margin_of_a_nested_code_block() {
    let _guard = lock_theme_test_state();
    let mut app = app_for(
        "- item

  ```rust
  let x = 1;
  ```",
    );
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let (_, row) = text_cell(&buffer, "let x");
    let (frame, _) = text_cell(&buffer, "┌─");
    let (copied, handled) = double_click(&mut app, (frame - 1, row), KeyModifiers::CONTROL);
    assert!(copied.is_empty(), "{copied:?}");
    assert!(!handled);
}

#[test]
fn expired_highlight_is_not_drawn() {
    let _guard = lock_theme_test_state();
    let mut app = app_for("Hello world");
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    let (column, row) = text_cell(&buffer, "world");
    double_click(&mut app, (column, row), KeyModifiers::NONE);
    let highlight = app_theme().markdown.search_highlight_bg;
    assert_eq!(draw(&mut app, WIDTH, HEIGHT)[(column, row)].bg, highlight);

    let ranges = app.copy_highlight().unwrap().0.clone();
    app.copy_highlight = Some((
        ranges,
        Instant::now() - Duration::from_millis(COPY_HIGHLIGHT_DURATION_MS),
    ));
    let after = draw(&mut app, WIDTH, HEIGHT);
    for x in 0..WIDTH {
        assert_ne!(after[(x, row)].bg, highlight, "column {x}");
    }
}

#[test]
fn word_copy_skips_footnote_references_but_keeps_math() {
    let _guard = lock_theme_test_state();
    assert_eq!(
        copy_at(
            "Text ref[^1] here.\n\n[^1]: note body",
            "ref",
            KeyModifiers::NONE
        ),
        ["ref"]
    );
    assert_eq!(copy_at("Area $x^2$ here", "x", KeyModifiers::NONE), ["x²"]);
}

#[test]
fn inline_chip_padding_is_highlighted_but_not_copied() {
    let _guard = lock_theme_test_state();
    for (source, text, modifiers, expected) in [
        (
            "Run `cargo test` now",
            "now",
            KeyModifiers::CONTROL,
            "Run cargo test now",
        ),
        (
            "`cargo` at start",
            "start",
            KeyModifiers::CONTROL,
            "cargo at start",
        ),
        (
            "Area $x^2$ and ==marked== end",
            "end",
            KeyModifiers::CONTROL,
            "Area x² and marked end",
        ),
        ("`cargo` at start", "cargo", KeyModifiers::NONE, "cargo"),
    ] {
        assert_eq!(
            copy_at(source, text, modifiers),
            [expected],
            "{source:?} {modifiers:?}"
        );
    }

    let highlight = app_theme().markdown.search_highlight_bg;
    for modifiers in [KeyModifiers::NONE, KeyModifiers::CONTROL] {
        let mut app = app_for("`cargo` at start");
        let buffer = draw(&mut app, WIDTH, HEIGHT);
        let (column, row) = text_cell(&buffer, "cargo");
        double_click(&mut app, (column, row), modifiers);
        let after = draw(&mut app, WIDTH, HEIGHT);
        assert_eq!(after[(column - 1, row)].bg, highlight, "{modifiers:?}");
        assert_eq!(after[(column + 5, row)].bg, highlight, "{modifiers:?}");
    }
}

#[test]
fn inline_chip_spaces_survive_paragraph_wrapping() {
    let _guard = lock_theme_test_state();
    let source = "Run `cargo test --all` then read the long explanation that follows here";
    let mut app = app_for(source);
    let buffer = draw(&mut app, WIDTH, HEIGHT);
    assert_ne!(
        text_cell(&buffer, "Run").1,
        text_cell(&buffer, "here").1,
        "source must wrap"
    );
    let (copied, _) = double_click(&mut app, text_cell(&buffer, "Run"), KeyModifiers::CONTROL);
    assert_eq!(
        copied,
        ["Run cargo test --all then read the long explanation that follows here"]
    );
    let (copied, _) = double_click(&mut app, text_cell(&buffer, "cargo"), KeyModifiers::NONE);
    assert_eq!(copied, ["cargo"]);
    let range = app.copy_highlight().unwrap().0[0];
    assert_eq!(range.end_col - range.start_col, " cargo".len());
}
