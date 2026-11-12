use super::{parse_markdown_at as parse_at, rendered_non_empty_lines, test_md_theme};
use crate::markdown::callouts::{callout_kind, default_label, parse_callout_marker, CalloutKind};
use crate::markdown::{line_plain_text, ParseResult, BLOCKQUOTE_BAR};
use ratatui::style::{Color, Modifier};
use ratatui::text::Line;

fn parse(src: &str) -> ParseResult {
    parse_at(src, 80, false)
}

fn rendered(src: &str) -> Vec<String> {
    rendered_non_empty_lines(&parse(src).lines)
}

fn line_with<'a>(lines: &'a [Line<'static>], text: &str) -> &'a Line<'static> {
    lines
        .iter()
        .find(|line| line_plain_text(line).contains(text))
        .unwrap_or_else(|| panic!("missing line containing {text:?}"))
}

fn bar_colors(line: &Line<'static>) -> Vec<Option<Color>> {
    line.spans
        .iter()
        .filter(|span| span.content == BLOCKQUOTE_BAR)
        .map(|span| span.style.fg)
        .collect()
}

#[test]
fn marker_parsing_handles_aliases_case_and_fold_markers() {
    for (name, kind) in [
        ("note", CalloutKind::Note),
        ("SUMMARY", CalloutKind::Abstract),
        ("tldr", CalloutKind::Abstract),
        ("Info", CalloutKind::Info),
        ("todo", CalloutKind::Todo),
        ("hint", CalloutKind::Tip),
        ("important", CalloutKind::Important),
        ("done", CalloutKind::Success),
        ("faq", CalloutKind::Question),
        ("attention", CalloutKind::Warning),
        ("CAUTION", CalloutKind::Caution),
        ("missing", CalloutKind::Failure),
        ("error", CalloutKind::Danger),
        ("bug", CalloutKind::Bug),
        ("example", CalloutKind::Example),
        ("cite", CalloutKind::Quote),
        ("whatever", CalloutKind::Note),
    ] {
        assert_eq!(callout_kind(name), kind, "{name}");
    }
    assert_eq!(default_label("NOTE"), "Note");
    assert_eq!(default_label("faq"), "Faq");

    let src = "> [!faq]- Title here  \n> body";
    let marker = parse_callout_marker(src, 0).expect("callout");
    assert_eq!(marker.kind, CalloutKind::Question);
    assert!(marker.has_title);
    assert_eq!(&src[marker.marker_end..marker.marker_end + 5], "Title");

    let marker = parse_callout_marker("  > [!tip]+   \n", 0).expect("callout");
    assert!(!marker.has_title);
    let nested = "> > [!todo] Inner";
    let marker = parse_callout_marker(nested, 2).expect("nested callout");
    assert_eq!(marker.kind, CalloutKind::Todo);

    for src in [
        "> [!]",
        "> [!tip",
        "> [!1x]",
        "> text [!tip]",
        "> [x]",
        "> > [!tip]",
    ] {
        assert!(parse_callout_marker(src, 0).is_none(), "{src:?}");
    }
}

#[test]
fn custom_title_replaces_default_label_in_both_soft_break_modes() {
    for keep in [false, true] {
        let result = parse_at("> [!tip] Use watch mode\n> Run it.\n", 80, keep);
        let lines = rendered_non_empty_lines(&result.lines);
        assert_eq!(lines[..2], ["▏ [*] Use watch mode", "▏ Run it."], "{keep}");
    }
}

#[test]
fn callout_variants_render_header_without_marker() {
    assert_eq!(rendered("> [!Warning] Title only\n"), ["▏ [!] Title only"]);
    assert_eq!(
        rendered("> [!danger]\n> Boom\n"),
        ["▏ [x] Danger", "▏ Boom"]
    );
    assert_eq!(
        rendered("> [!faq]- Folded\n> body\n"),
        ["▏ [?] Folded", "▏ body"]
    );
    assert_eq!(rendered("> [!faq]+\n> body\n"), ["▏ [?] Faq", "▏ body"]);

    let result = parse("> [!foo]\n> body\n");
    let header = line_with(&result.lines, "Foo");
    assert_eq!(line_plain_text(header), "▏ [i] Foo");
    assert_eq!(bar_colors(header), [Some(test_md_theme().alert_note)]);
}

#[test]
fn gfm_semantics_kept_for_important_and_caution() {
    let theme = test_md_theme();
    for (src, color) in [
        ("> [!IMPORTANT]\n> x\n", theme.alert_important),
        ("> [!caution]\n> x\n", theme.alert_caution),
        ("> [!quote] Said\n> x\n", theme.blockquote_marker),
    ] {
        let result = parse(src);
        let header = &result.lines[0];
        assert_eq!(bar_colors(header), [Some(color)], "{src:?}");
        assert_eq!(header.spans[1].style.fg, Some(color), "{src:?}");
    }
}

#[test]
fn title_keeps_inline_code_and_link_styles() {
    let theme = test_md_theme();
    let result = parse("> [!tip] Run *now* with `leaf` and [docs](http://x)\n> body\n");
    let header = &result.lines[0];
    assert_eq!(
        line_plain_text(header),
        "▏ [*] Run now with  leaf  and #docs"
    );
    let now = header.spans.iter().find(|s| s.content == "now").unwrap();
    assert_eq!(now.style.fg, Some(theme.alert_tip));
    assert!(now.style.add_modifier.contains(Modifier::BOLD));
    let code = header
        .spans
        .iter()
        .find(|s| s.content.contains("leaf"))
        .unwrap();
    assert_eq!(code.style.bg, Some(theme.inline_code_bg));
    assert!(result.link_spans.iter().any(|span| span.line_idx == 0));
}

#[test]
fn long_title_wraps_with_bars() {
    let result = parse_at(
        "> [!note] alpha beta gamma delta epsilon zeta eta theta\n> body\n",
        30,
        false,
    );
    let lines = rendered_non_empty_lines(&result.lines);
    assert!(lines.len() >= 3, "{lines:?}");
    assert!(lines[0].starts_with("▏ [i] alpha"), "{lines:?}");
    assert!(
        lines[1].starts_with("▏ ") && !lines[1].contains("[i]"),
        "{lines:?}"
    );
    assert_eq!(lines.last().unwrap(), "▏ body");
}

#[test]
fn nested_callouts_keep_their_own_bar_colors() {
    let theme = test_md_theme();
    let src = "> [!WARNING]\n> > [!NOTE]\n> > inner\n>\n> after\n";
    let result = parse(src);
    assert_eq!(
        rendered_non_empty_lines(&result.lines),
        [
            "▏ [!] Warning",
            "▏ ▏ [i] Note",
            "▏ ▏ inner",
            "▏ ",
            "▏ after"
        ]
    );
    let warn = Some(theme.alert_warning);
    let note = Some(theme.alert_note);
    assert_eq!(
        bar_colors(line_with(&result.lines, "[i] Note")),
        [warn, note]
    );
    assert_eq!(bar_colors(line_with(&result.lines, "inner")), [warn, note]);
    assert_eq!(bar_colors(line_with(&result.lines, "after")), [warn]);
}

#[test]
fn blocks_inside_callout_use_callout_bar_color() {
    let theme = test_md_theme();
    let src = "> [!WARNING]\n> text\n>\n> ```\n> code\n> ```\n>\n> $$x$$\n>\n> | a | b |\n> |---|---|\n> | 1 | 2 |\n";
    let result = parse_at(src, 60, false);
    let warn = Some(theme.alert_warning);
    for text in ["code", "│1│ x", "│ a │ b │", "╞═══╪═══╡", "└───┴───┘"]
    {
        let line = line_with(&result.lines, text);
        assert_eq!(bar_colors(line), [warn], "{text:?}");
    }
}

#[test]
fn table_inside_blockquote_respects_prefix_width() {
    let src = "> | a | b |\n> |---|---|\n> | alpha beta gamma delta | epsilon zeta eta theta |\n";
    let result = parse_at(src, 30, false);
    for line in rendered_non_empty_lines(&result.lines) {
        assert!(line.starts_with("▏ "), "{line:?}");
        assert!(crate::markdown::display_width(&line) <= 30, "{line:?}");
    }
}

#[test]
fn br_tag_ends_custom_title() {
    assert_eq!(rendered("> [!tip] a<br>b\n> c\n"), ["▏ [*] a", "▏ b c"]);
    assert_eq!(rendered("> [!tip] a<BR />b\n"), ["▏ [*] a", "▏ b"]);
    assert_eq!(
        rendered("> [!tip] a<br class=\"x\">b\n"),
        ["▏ [*] a", "▏ b"]
    );
    assert_eq!(rendered("> [!tip] a</br>b\n"), ["▏ [*] a", "▏ b"]);
}

#[test]
fn marker_line_consumed_as_reference_definition_is_a_plain_quote() {
    let result = parse("> [!x]: http://example.com\n> body\n");
    let lines = rendered_non_empty_lines(&result.lines);
    assert_eq!(lines, ["▏ body"]);
    let body = line_with(&result.lines, "body");
    assert_eq!(bar_colors(body), [Some(test_md_theme().blockquote_marker)]);
}
