use super::common::app_helpers::{draw_app, markdown_app, status_text};
use super::{
    lock_theme_test_state, parse_markdown_at as parse_at, rendered_non_empty_lines,
    test_syntax_set, test_theme_set,
};
use crate::app::App;
use crate::markdown::ParseResult;

fn all_lines(app: &App) -> &[ratatui::text::Line<'static>] {
    app.visible_lines(0, app.total())
}

fn parse(src: &str, keep_soft_breaks: bool) -> ParseResult {
    parse_at(src, 80, keep_soft_breaks)
}

fn rendered(src: &str, keep_soft_breaks: bool) -> Vec<String> {
    rendered_non_empty_lines(&parse(src, keep_soft_breaks).lines)
}

#[test]
fn soft_breaks_are_joined_or_kept() {
    let src = "alpha beta\ngamma delta\nepsilon";
    assert_eq!(rendered(src, false), ["alpha beta gamma delta epsilon"]);
    assert_eq!(
        rendered(src, true),
        ["alpha beta", "gamma delta", "epsilon"]
    );
}

#[test]
fn joined_paragraph_is_rewrapped_as_one_logical_line() {
    let src = "one two three four five six\nseven eight nine ten";
    let parsed = parse_at(src, 20, false);
    let rows: Vec<(String, usize)> = parsed
        .lines
        .iter()
        .map(crate::line_plain_text)
        .zip(parsed.line_number_map.iter().copied())
        .filter(|(text, _)| !text.is_empty())
        .collect();
    assert!(rows.len() > 1, "paragraph must wrap");
    let words: Vec<&str> = rows
        .iter()
        .flat_map(|(text, _)| text.split_whitespace())
        .collect();
    assert_eq!(
        words.join(" "),
        "one two three four five six seven eight nine ten"
    );
    assert!(
        rows.iter().all(|(text, _)| text.chars().count() <= 20),
        "{rows:?}"
    );
    assert!(
        rows.iter().all(|(_, number)| *number == rows[0].1),
        "{rows:?}"
    );
}

#[test]
fn hard_breaks_are_kept_in_both_modes() {
    for src in ["alpha  \nbeta", "alpha\\\nbeta", "alpha<br>beta"] {
        for keep in [false, true] {
            assert_eq!(
                rendered(src, keep),
                ["alpha", "beta"],
                "{src:?} keep={keep}"
            );
        }
    }
}

#[test]
fn soft_breaks_between_cjk_characters_join_without_space() {
    assert_eq!(rendered("中文\n中文", false), ["中文中文"]);
    assert_eq!(rendered("日本語。\nかな", false), ["日本語。かな"]);
    assert_eq!(rendered("中文\nabc", false), ["中文 abc"]);
    assert_eq!(rendered("abc\n中文", false), ["abc 中文"]);
}

#[test]
fn soft_breaks_between_hangul_keep_the_space() {
    assert_eq!(rendered("한국\n어", false), ["한국 어"]);
}

#[test]
fn crlf_soft_breaks_are_joined() {
    assert_eq!(rendered("alpha\r\nbeta", false), ["alpha beta"]);
}

#[test]
fn soft_breaks_in_blockquote_are_joined() {
    assert_eq!(rendered("> alpha\n> beta", false), ["▏ alpha beta"]);
}

#[test]
fn soft_breaks_in_blockquote_with_cjk_skip_the_quote_marker() {
    assert_eq!(rendered("> 中文\n> 中文", false), ["▏ 中文中文"]);
}

#[test]
fn soft_breaks_in_list_item_are_joined() {
    assert_eq!(rendered("- alpha\n  beta", false), ["• alpha beta"]);
}

#[test]
fn soft_break_keeps_bold_style() {
    let parsed = parse("**alpha\nbeta**", false);
    let line = parsed
        .lines
        .iter()
        .find(|line| crate::line_plain_text(line).contains("alpha"))
        .unwrap();
    assert_eq!(crate::line_plain_text(line), "alpha beta");
    let bold = line.spans[0].style;
    assert!(line.spans.iter().all(|span| span.style == bold), "{line:?}");
}

#[test]
fn soft_break_inside_link_keeps_the_link_id() {
    let parsed = parse("[alpha\nbeta](https://example.com)", false);
    assert_eq!(parsed.link_urls, ["https://example.com"]);
    let line = &parsed.lines[parsed.link_spans[0].line_idx];
    let text = crate::line_plain_text(line);
    assert!(text.contains("alpha beta"), "{text:?}");
    let covered: usize = parsed
        .link_spans
        .iter()
        .map(|span| span.end_col - span.start_col)
        .sum();
    assert!(
        covered >= "alpha beta".len(),
        "link must cover the joined space"
    );
}

#[test]
fn multiline_setext_heading_is_one_heading_and_one_toc_entry() {
    let parsed = parse("Title\nsuite\n===\n\nbody", false);
    assert_eq!(parsed.toc.len(), 1);
    assert_eq!(parsed.toc[0].title, "Title suite");
    let lines = rendered_non_empty_lines(&parsed.lines);
    assert!(
        lines.iter().any(|line| line.contains("Title suite")),
        "{lines:?}"
    );
}

#[test]
fn plain_text_extension_detection() {
    for ext in ["txt", "log", "csv", "rs"] {
        assert!(App::is_plain_text_extension(ext), "{ext}");
    }
    for ext in ["", "md", "MD", "markdown"] {
        assert!(!App::is_plain_text_extension(ext), "{ext}");
    }
}

#[test]
fn effective_keep_soft_breaks_follows_preference_file_type_and_override() {
    let mut app = markdown_app("alpha", 80, "doc.md", Some("/docs/doc.md"));
    assert!(app.keep_soft_breaks(), "soft breaks are kept by default");

    app.set_keep_soft_breaks(false);
    assert!(!app.keep_soft_breaks(), "preference applies");

    app.set_plain_text_file(true);
    assert!(app.keep_soft_breaks(), "plain text files keep soft breaks");

    app.toggle_soft_breaks(&test_syntax_set(), test_theme_set());
    assert!(!app.keep_soft_breaks(), "override wins over file type");

    app.set_plain_text_file(false);
    app.set_keep_soft_breaks(true);
    assert!(!app.keep_soft_breaks(), "override wins over preference");
}

#[test]
fn toggle_soft_breaks_rerenders_and_flashes_success() {
    let _guard = lock_theme_test_state();
    let src = "alpha beta\ngamma delta";
    let mut app = markdown_app(src, 80, "doc.md", Some("/docs/doc.md"));
    let ss = test_syntax_set();
    let themes = test_theme_set();

    app.toggle_soft_breaks(&ss, themes);
    assert!(!app.keep_soft_breaks());
    let buffer = draw_app(&mut app, 80, 10);
    assert!(status_text(&buffer).contains("Soft breaks joined"));
    assert_eq!(
        rendered_non_empty_lines(all_lines(&app)),
        ["alpha beta gamma delta"]
    );

    app.toggle_soft_breaks(&ss, themes);
    assert!(app.keep_soft_breaks());
    let buffer = draw_app(&mut app, 80, 10);
    assert!(status_text(&buffer).contains("Soft breaks kept"));
    assert_eq!(
        rendered_non_empty_lines(all_lines(&app)),
        ["alpha beta", "gamma delta"]
    );
}

#[test]
fn toggle_soft_breaks_keeps_the_reading_position() {
    let _guard = lock_theme_test_state();
    let mut src = String::new();
    for i in 0..40 {
        src.push_str(&format!(
            "paragraph {i} line one\nparagraph {i} line two\n\n"
        ));
    }
    let mut app = markdown_app(&src, 80, "doc.md", Some("/docs/doc.md"));
    draw_app(&mut app, 80, 10);
    let ss = test_syntax_set();
    let themes = test_theme_set();

    let target = all_lines(&app)
        .iter()
        .position(|line| crate::line_plain_text(line).starts_with("paragraph 20 "))
        .unwrap();
    app.scroll_to(target);

    app.toggle_soft_breaks(&ss, themes);
    let top = crate::line_plain_text(&all_lines(&app)[app.scroll()]);
    assert!(top.starts_with("paragraph 20 line one"), "{top:?}");

    app.toggle_soft_breaks(&ss, themes);
    let top = crate::line_plain_text(&all_lines(&app)[app.scroll()]);
    assert_eq!(top, "paragraph 20 line one");
}

#[test]
fn toggle_to_joined_from_mid_paragraph_keeps_the_current_paragraph() {
    let _guard = lock_theme_test_state();
    let mut src = String::new();
    for i in 0..40 {
        src.push_str(&format!(
            "paragraph {i} line one\nparagraph {i} line two\nparagraph {i} line three\n\n"
        ));
    }
    let mut app = markdown_app(&src, 80, "doc.md", Some("/docs/doc.md"));
    draw_app(&mut app, 80, 10);
    let ss = test_syntax_set();
    let themes = test_theme_set();

    assert!(app.keep_soft_breaks());
    let target = all_lines(&app)
        .iter()
        .position(|line| crate::line_plain_text(line) == "paragraph 20 line two")
        .unwrap();
    app.scroll_to(target);

    app.toggle_soft_breaks(&ss, themes);
    assert!(!app.keep_soft_breaks());
    let top = crate::line_plain_text(&all_lines(&app)[app.scroll()]);
    assert!(top.starts_with("paragraph 20 line one"), "{top:?}");
}
