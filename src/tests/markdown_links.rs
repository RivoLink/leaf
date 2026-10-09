use super::{test_assets, test_md_theme};
use crate::markdown::{
    display_width, highlight_line, parse_markdown, parse_markdown_with_width, resolve_syntax,
    LinkId, LinkSpan,
};
use crate::theme::app_theme;
use ratatui::{
    style::Style,
    text::{Line, Span},
};
use syntect::parsing::SyntaxSet;

fn adversarial_table_fixture() -> (Vec<Line<'static>>, Vec<LinkSpan>, Vec<String>) {
    let (ss, theme) = super::test_assets();
    // A wraps in the first cell before C; parser registration is A/C/B while visual rows are A/B/C.
    let source = "| Left | Right |\n| :---: | ---: |\n| [A A A A A A A A A A A A A A A A](https://example.test/a) [C](https://example.test/c) | [B](https://example.test/b) |\n";
    let parsed = parse_markdown_with_width(
        source,
        &ss,
        &theme,
        36,
        &super::test_md_theme(),
        false,
        true,
    );
    (parsed.lines, parsed.link_spans, parsed.link_urls)
}

fn span_destinations<'a>(spans: &[LinkSpan], urls: &'a [String]) -> Vec<&'a str> {
    spans
        .iter()
        .map(|span| urls[span.link_id.0].as_str())
        .collect()
}

fn table_link_marker_positions(lines: &[Line<'_>]) -> Vec<(usize, usize, char)> {
    let link_icon = app_theme().markdown.link_icon;
    lines
        .iter()
        .enumerate()
        .flat_map(|(line_idx, line)| {
            let text: String = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();
            let mut col = 0usize;
            let mut markers = Vec::new();
            for span in &line.spans {
                let content = span.content.as_ref();
                if content == "#" && span.style.fg == Some(link_icon) {
                    let label = text
                        .chars()
                        .nth(col + 1)
                        .expect("fixture marker should be followed by a label");
                    markers.push((line_idx, col, label));
                }
                col += display_width(content);
            }
            markers
        })
        .collect()
}

#[test]
fn blockquote_bold_link_preserves_link_color() {
    let (ss, theme) = test_assets();
    let src = "> text [**lien bold**](https://rivolink.mg)\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let app_theme = app_theme();
    let theme_colors = &app_theme.markdown;

    let bq_line = &lines[0];
    let link_span = bq_line.spans.iter().find(|s| s.content.contains("lien"));
    assert!(link_span.is_some(), "should find 'lien' span");
    let span = link_span.unwrap();
    assert_eq!(
        span.style.fg,
        Some(theme_colors.link_text),
        "bold link in blockquote should preserve link_text color"
    );
}

#[test]
fn link_spans_detected_for_all_link_types() {
    let (ss, theme) = test_assets();
    let md = "\
[Simple](https://example.com/simple)

**[Bold link](https://example.com/bold)**

*[Italic link](https://example.com/italic)*

~~[Strike link](https://example.com/strike)~~

[Internal](#section)

### [Heading link](https://example.com/heading)

> [Blockquote link](https://example.com/quote)

[A](https://example.com/a) and [B](https://example.com/b)
";
    let parsed = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true);
    let link_spans = parsed.link_spans;
    let urls = span_destinations(&link_spans, &parsed.link_urls);

    assert!(
        urls.contains(&"https://example.com/simple"),
        "simple link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/bold"),
        "bold link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/italic"),
        "italic link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/strike"),
        "strikethrough link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"#section"),
        "internal link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/heading"),
        "heading link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/quote"),
        "blockquote link missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/a"),
        "multi-link A missing: {urls:?}"
    );
    assert!(
        urls.contains(&"https://example.com/b"),
        "multi-link B missing: {urls:?}"
    );

    for ls in &link_spans {
        assert!(
            ls.end_col > ls.start_col,
            "link {:?} has zero width (start={} end={})",
            parsed.link_urls[ls.link_id.0],
            ls.start_col,
            ls.end_col,
        );
    }
}

#[test]
fn link_spans_in_table_are_detected() {
    let (ss, theme) = test_assets();
    let md = "\
| Name | Link |
|------|------|
| Test | [example](https://example.com/table) |
";
    let parsed = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true);
    let link_spans = parsed.link_spans;
    let urls = span_destinations(&link_spans, &parsed.link_urls);
    assert!(
        urls.contains(&"https://example.com/table"),
        "table link missing: {urls:?}"
    );
}

#[test]
fn wrapped_repeated_destinations_keep_distinct_ids_and_continuations() {
    let (ss, theme) = test_assets();
    let md = "[one two three four five six seven](https://example.test/repeat) and [again](https://example.test/repeat)";
    let parsed = parse_markdown_with_width(md, &ss, &theme, 18, &test_md_theme(), false, true);
    assert_eq!(parsed.link_urls.len(), 2);
    assert_eq!(parsed.link_urls[0], parsed.link_urls[1]);
    let first_rows: Vec<String> = parsed
        .link_spans
        .iter()
        .filter(|span| span.link_id == LinkId(0))
        .map(|span| {
            crate::markdown::line_plain_text(&parsed.lines[span.line_idx])
                .chars()
                .skip(span.start_col)
                .take(span.end_col - span.start_col)
                .collect()
        })
        .collect();
    assert_eq!(first_rows, ["#one two three ", "four five six ", "seven"]);
}

#[test]
fn mixed_style_mapping_does_not_depend_on_link_color() {
    let (ss, theme) = test_assets();
    let mut md_theme = test_md_theme();
    md_theme.link_text = md_theme.text;
    let parsed = parse_markdown(
        "[**bold** and `code` and ==mark== and $\\alpha$](https://example.test/mixed) tail",
        &ss,
        &theme,
        &md_theme,
        false,
        true,
    );
    assert_eq!(parsed.link_spans.len(), 1);
    let span = &parsed.link_spans[0];
    let text = crate::markdown::line_plain_text(&parsed.lines[span.line_idx]);
    let covered: String = text
        .chars()
        .skip(span.start_col)
        .take(span.end_col - span.start_col)
        .collect();
    assert_eq!(covered, "#bold and  code  and  mark  and  α ");
}

#[test]
fn html_style_buffer_around_link_keeps_label_owned() {
    for md in [
        "<mark>[label](https://a.example)</mark> [next](https://b.example)\n",
        "<code>[label](https://a.example)</code> [next](https://b.example)\n",
        "| x |\n| --- |\n| <mark>[label](https://a.example)</mark> [next](https://b.example) |\n",
    ] {
        let links = link_texts(md, 80);
        assert_eq!(
            links,
            vec![
                ("# label ".into(), "https://a.example".into()),
                ("#next".into(), "https://b.example".into()),
            ],
            "{md}"
        );
    }
}

#[test]
fn highlight_line_single_match() {
    let theme = test_md_theme();
    let line_bg = theme.search_highlight_bg;
    let match_bg = theme.search_match_bg;
    let line = Line::from(vec![Span::raw("hello world")]);
    let result = highlight_line(&line, &theme, "world");
    assert_eq!(result.spans.len(), 2);
    assert_eq!(result.spans[0].content.as_ref(), "hello ");
    assert_eq!(result.spans[0].style.bg, Some(line_bg));
    assert_eq!(result.spans[1].content.as_ref(), "world");
    assert_eq!(result.spans[1].style.bg, Some(match_bg));
    assert!(result.spans[1]
        .style
        .add_modifier
        .contains(ratatui::style::Modifier::BOLD));
}

#[test]
fn highlight_line_multiple_matches() {
    let theme = test_md_theme();
    let match_bg = theme.search_match_bg;
    let line = Line::from(vec![Span::raw("abcabcabc")]);
    let result = highlight_line(&line, &theme, "abc");
    assert_eq!(result.spans.len(), 3);
    for span in &result.spans {
        assert_eq!(span.content.as_ref(), "abc");
        assert_eq!(span.style.bg, Some(match_bg));
        assert!(span
            .style
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD));
    }
}

#[test]
fn highlight_line_case_insensitive() {
    let theme = test_md_theme();
    let line_bg = theme.search_highlight_bg;
    let match_bg = theme.search_match_bg;
    let line = Line::from(vec![Span::raw("Hello World")]);
    let result = highlight_line(&line, &theme, "hello");
    assert_eq!(result.spans.len(), 2);
    assert_eq!(result.spans[0].content.as_ref(), "Hello");
    assert_eq!(result.spans[0].style.bg, Some(match_bg));
    assert!(result.spans[0]
        .style
        .add_modifier
        .contains(ratatui::style::Modifier::BOLD));
    assert_eq!(result.spans[1].content.as_ref(), " World");
    assert_eq!(result.spans[1].style.bg, Some(line_bg));
    assert!(!result.spans[1]
        .style
        .add_modifier
        .contains(ratatui::style::Modifier::BOLD));
}

#[test]
fn highlight_line_cross_span() {
    let theme = test_md_theme();
    let line_bg = theme.search_highlight_bg;
    let match_bg = theme.search_match_bg;
    let bold = Style::default().add_modifier(ratatui::style::Modifier::BOLD);
    let line = Line::from(vec![Span::styled("hel", bold), Span::raw("lo world")]);
    let result = highlight_line(&line, &theme, "hello");
    assert_eq!(result.spans[0].content.as_ref(), "hel");
    assert_eq!(result.spans[0].style.bg, Some(match_bg));
    assert!(result.spans[0]
        .style
        .add_modifier
        .contains(ratatui::style::Modifier::BOLD));
    assert_eq!(result.spans[1].content.as_ref(), "lo");
    assert_eq!(result.spans[1].style.bg, Some(match_bg));
    assert!(result.spans[1]
        .style
        .add_modifier
        .contains(ratatui::style::Modifier::BOLD));
    assert_eq!(result.spans[2].content.as_ref(), " world");
    assert_eq!(result.spans[2].style.bg, Some(line_bg));
}

#[test]
fn highlight_line_no_match_returns_clone() {
    let theme = test_md_theme();
    let line = Line::from(vec![Span::raw("hello world")]);
    let result = highlight_line(&line, &theme, "xyz");
    assert_eq!(result.spans.len(), 1);
    assert_eq!(result.spans[0].content.as_ref(), "hello world");
    assert_eq!(result.spans[0].style.bg, None);
}

#[test]
fn resolve_syntax_supports_common_language_aliases() {
    let ss = SyntaxSet::load_defaults_newlines();

    assert_eq!(
        resolve_syntax("py", &ss).name,
        resolve_syntax("python", &ss).name
    );
    assert_eq!(
        resolve_syntax("cpp", &ss).name,
        resolve_syntax("c++", &ss).name
    );
    assert_eq!(resolve_syntax("json", &ss).name, "JSON");
    assert_eq!(resolve_syntax("json5", &ss).name, "JSON");
    assert_eq!(
        resolve_syntax("ps1", &ss).name,
        resolve_syntax("powershell", &ss).name
    );

    for tag in &["kotlin", "toml", "jsx", "dockerfile"] {
        assert_ne!(
            resolve_syntax(tag, &ss).name,
            "Plain Text",
            "{tag} should not fall back to Plain Text"
        );
    }
    assert_eq!(
        resolve_syntax("kt", &ss).name,
        resolve_syntax("kotlin", &ss).name
    );
    assert_eq!(
        resolve_syntax("docker", &ss).name,
        resolve_syntax("dockerfile", &ss).name
    );
    assert_eq!(
        resolve_syntax("pwsh", &ss).name,
        resolve_syntax("ps1", &ss).name
    );
}

#[test]
fn resolve_syntax_php_uses_php_source() {
    let ss = SyntaxSet::load_defaults_newlines();

    for tag in &["php", "PHP", "php3", "php4", "php5", "php7", "phtml"] {
        assert_eq!(
            resolve_syntax(tag, &ss).name,
            "PHP Source",
            "{tag} should resolve to PHP Source"
        );
    }
}

#[test]
fn php_code_block_without_open_tag_is_highlighted() {
    let (ss, theme) = test_assets();
    let md = "```php\nforeach ($map as $item) {\n    $scores[] = $item ?? 0;\n}\n```\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, true).into();

    let span_fg = |needle: &str| {
        lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.contains(needle))
            .and_then(|s| s.style.fg)
    };
    let keyword_fg = span_fg("foreach");
    let variable_fg = span_fg("scores");

    assert!(keyword_fg.is_some(), "should find 'foreach' span");
    assert!(variable_fg.is_some(), "should find 'scores' span");
    assert_ne!(
        keyword_fg, variable_fg,
        "php block without <?php should highlight keywords and variables differently"
    );
}

struct LinkMarkerGuard;

impl LinkMarkerGuard {
    fn set(marker: &str) -> Self {
        crate::markdown::set_link_marker(marker);
        Self
    }
}

impl Drop for LinkMarkerGuard {
    fn drop(&mut self) {
        crate::markdown::set_link_marker(crate::markdown::DEFAULT_LINK_MARKER);
    }
}

fn link_texts(md: &str, width: usize) -> Vec<(String, String)> {
    let (ss, theme) = test_assets();
    let parsed = parse_markdown_with_width(md, &ss, &theme, width, &test_md_theme(), false, true);
    parsed
        .link_spans
        .iter()
        .map(|ls| {
            let text = crate::markdown::line_plain_text(&parsed.lines[ls.line_idx]);
            let clicked: String = text
                .chars()
                .skip(ls.start_col)
                .take(ls.end_col - ls.start_col)
                .collect();
            let destination = parsed.link_urls[ls.link_id.0].clone();
            (clicked, destination)
        })
        .collect()
}

#[test]
fn custom_link_prefix_replaces_marker() {
    let _guard = LinkMarkerGuard::set("→");
    let links = link_texts("see [leaf](https://a.example)\n", 80);
    assert_eq!(links, vec![("→leaf".into(), "https://a.example".into())]);
}

#[test]
fn empty_link_prefix_keeps_links_clickable() {
    let _guard = LinkMarkerGuard::set("");
    let md = "\
aaaaaaaaaaaaaaaaaaaaaaaaaaa [xyzxyzxyz](https://a.example) and [second](https://b.example)

| col |
| --- |
| [cell](https://c.example) |
";
    let links = link_texts(md, 30);
    let expected: Vec<(String, String)> = [
        ("xyzxyzxyz", "https://a.example"),
        ("second", "https://b.example"),
        ("cell", "https://c.example"),
    ]
    .into_iter()
    .map(|(t, u)| (t.into(), u.into()))
    .collect();
    assert_eq!(links, expected);
}

#[test]
fn empty_link_prefix_keeps_wrapped_table_links_clickable() {
    let _guard = LinkMarkerGuard::set("");
    let md = "\
| a | b |
| --- | --- |
| x | lead [label](https://a.example) and [averyveryverylonglabel](https://b.example) |
";
    let links = link_texts(md, 24);
    let mut urls: Vec<&str> = links.iter().map(|(_, u)| u.as_str()).collect();
    urls.dedup();
    assert_eq!(
        urls,
        ["https://a.example", "https://b.example"],
        "{links:?}"
    );
    assert!(links.iter().all(|(t, _)| !t.trim().is_empty()), "{links:?}");
    // The wrapped label keeps one range per visual row, so every row stays clickable.
    let wrapped_label: String = links
        .iter()
        .filter(|(_, u)| u == "https://b.example")
        .map(|(t, _)| t.as_str())
        .collect();
    assert_eq!(wrapped_label, "averyveryverylonglabel", "{links:?}");
}

#[test]
fn empty_link_prefix_keeps_code_label_clickable() {
    let _guard = LinkMarkerGuard::set("");
    let links = link_texts("see [`code`](https://a.example) end\n", 80);
    assert_eq!(links, vec![(" code ".into(), "https://a.example".into())]);
}

#[test]
fn display_math_keeps_link_ranges_of_the_flushed_paragraph() {
    assert_eq!(
        link_texts(
            "[a](https://a.test) then $$x^2$$ and [b](https://b.test) after.\n",
            80
        ),
        vec![
            ("#a".into(), "https://a.test".into()),
            ("#b".into(), "https://b.test".into()),
        ]
    );
    let _guard = LinkMarkerGuard::set("");
    for md in [
        "para\n\n[ ](https://x.test) $$x$$\n",
        "> para\n>\n> [ ](https://x.test) $$x$$\n",
    ] {
        assert_eq!(link_texts(md, 40), Vec::<(String, String)>::new(), "{md}");
    }
}

#[test]
fn empty_link_prefix_keeps_urls_aligned_after_empty_label() {
    let _guard = LinkMarkerGuard::set("");
    let md =
        "[](https://a.example) [label](https://b.example) some filler words to force wrapping\n";
    let links = link_texts(md, 20);
    assert_eq!(
        links.last(),
        Some(&("label".into(), "https://b.example".into()))
    );
}

#[test]
fn adversarial_table_fixture_preserves_visual_destination_order() {
    const A: &str = "https://example.test/a";
    const B: &str = "https://example.test/b";
    const C: &str = "https://example.test/c";

    let (lines, link_spans, urls) = adversarial_table_fixture();
    let markers = table_link_marker_positions(&lines);
    let labels: Vec<char> = markers.iter().map(|(_, _, label)| *label).collect();
    assert_eq!(labels, vec!['A', 'B', 'C']);
    assert!(
        markers[2].0 > markers[0].0,
        "C follows A on a continuation row"
    );

    for ((line, column, _), destination) in markers.iter().zip([A, B, C]) {
        let range = link_spans
            .iter()
            .find(|span| {
                span.line_idx == *line && span.start_col <= *column && *column < span.end_col
            })
            .expect("every visible marker must have exact ownership");
        assert_eq!(urls[range.link_id.0], destination);
    }
    let mut seen = std::collections::HashSet::new();
    let visual_destinations: Vec<_> = link_spans
        .iter()
        .filter(|span| seen.insert(span.link_id))
        .map(|span| urls[span.link_id.0].as_str())
        .collect();
    assert_eq!(visual_destinations, vec![A, B, C]);
    assert!(
        link_spans
            .iter()
            .filter(|span| urls[span.link_id.0] == A)
            .count()
            > 1
    );
}
