use super::{test_assets, test_md_theme};
use crate::app::App;
use crate::markdown::parse_markdown;
use crate::*;
use ratatui::layout::Rect;

fn toc(entries: &[(u8, usize)]) -> Vec<TocEntry> {
    entries
        .iter()
        .enumerate()
        .map(|(i, (level, line))| TocEntry {
            level: *level,
            title: format!("Section {}", i + 1),
            line: *line,
        })
        .collect()
}

fn make_app_with_toc(total_lines: usize, viewport_height: u16, toc: Vec<TocEntry>) -> App {
    let (ss, theme) = test_assets();
    let md = (0..total_lines)
        .map(|_| "line")
        .collect::<Vec<_>>()
        .join("\n");
    let (lines, _, _, _) = parse_markdown(&md, &ss, &theme, &test_md_theme(), false);
    let mut app = App::new(lines, toc, "test".to_string(), false, false, None, None);
    app.content_area = Rect::new(0, 0, 80, viewport_height);
    app
}

#[test]
fn active_toc_highlights_last_header_when_short_section_at_bottom() {
    let mut app = make_app_with_toc(100, 15, toc(&[(2, 0), (2, 30), (2, 70), (2, 95)]));
    app.scroll_bottom();
    assert_eq!(app.active_toc_index(), Some(3));
}

#[test]
fn active_toc_unchanged_when_document_fits_in_viewport() {
    let mut app = make_app_with_toc(10, 20, toc(&[(2, 0), (2, 5)]));
    app.scroll_bottom();
    assert_eq!(app.active_toc_index(), Some(0));
}

#[test]
fn active_toc_last_header_with_long_section_uses_existing_logic() {
    let mut app = make_app_with_toc(100, 15, toc(&[(2, 0), (2, 30), (2, 50)]));
    app.scroll_bottom();
    assert_eq!(app.active_toc_index(), Some(2));
}

#[test]
fn active_toc_intermediate_header() {
    let mut app = make_app_with_toc(100, 15, toc(&[(2, 0), (2, 30), (2, 70)]));
    app.scroll = 40;
    assert_eq!(app.active_toc_index(), Some(1));
}

#[test]
fn active_toc_empty_toc_returns_none() {
    let app = make_app_with_toc(50, 15, vec![]);
    assert_eq!(app.active_toc_index(), None);
}

#[test]
fn active_toc_single_header() {
    let app = make_app_with_toc(50, 15, toc(&[(2, 0)]));
    assert_eq!(app.active_toc_index(), Some(0));
}

#[test]
fn toc_only_includes_first_two_heading_levels() {
    let (ss, theme) = test_assets();
    let (_, toc, _, _) = parse_markdown(
        "# One\n## Two\n### Three\n#### Four\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
    );

    assert_eq!(toc.len(), 3);
    assert_eq!(toc[0].level, 1);
    assert_eq!(toc[1].level, 2);
    assert_eq!(toc[2].level, 3);
}

#[test]
fn frontmatter_is_ignored_in_toc() {
    let (ss, theme) = test_assets();
    let src = "---\ntitle: Demo\nowner: me\n---\n# Visible\nBody\n";
    let (_, toc, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false);

    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].title, "Visible");
}

#[test]
fn toc_hides_single_h1_when_h2_entries_exist() {
    let toc = vec![
        TocEntry {
            level: 1,
            title: "Doc Title".to_string(),
            line: 0,
        },
        TocEntry {
            level: 2,
            title: "Install".to_string(),
            line: 10,
        },
    ];

    let (shift, hide_root) = toc_promotion(&toc);
    assert!(hide_root);
    assert_eq!(shift, 1);
    assert_eq!(toc_display_level(2, shift, hide_root), 1);
    assert_eq!(toc_display_level(3, shift, hide_root), 2);
}

#[test]
fn toc_keeps_single_h1_when_no_h2_entries_exist() {
    let toc = vec![TocEntry {
        level: 1,
        title: "Doc Title".to_string(),
        line: 0,
    }];

    let (_shift, hide_root) = toc_promotion(&toc);
    assert!(!hide_root);
}

#[test]
fn toc_promotes_h2_when_document_has_no_h1() {
    let toc = vec![
        TocEntry {
            level: 2,
            title: "Build & install".to_string(),
            line: 0,
        },
        TocEntry {
            level: 3,
            title: "Android".to_string(),
            line: 4,
        },
    ];

    let (shift, hide_root) = toc_promotion(&toc);
    assert!(!hide_root);
    assert_eq!(shift, 1);
    assert_eq!(toc_display_level(2, shift, hide_root), 1);
    assert_eq!(toc_display_level(3, shift, hide_root), 2);
    let normalized = normalize_toc(toc);
    assert_eq!(normalized.len(), 2);
    assert_eq!(normalized[0].level, 2);
    assert_eq!(normalized[1].level, 3);
}

#[test]
fn toc_promotes_h3_when_document_has_no_h1_or_h2() {
    let toc = vec![
        TocEntry {
            level: 3,
            title: "Section A".to_string(),
            line: 0,
        },
        TocEntry {
            level: 4,
            title: "Subsection".to_string(),
            line: 5,
        },
    ];

    let (shift, hide_root) = toc_promotion(&toc);
    assert!(!hide_root);
    assert_eq!(shift, 2);
    assert_eq!(toc_display_level(3, shift, hide_root), 1);
    assert_eq!(toc_display_level(4, shift, hide_root), 2);
}

#[test]
fn toc_promotes_single_h3_to_root_without_hiding() {
    let toc = vec![
        TocEntry {
            level: 3,
            title: "Only H3".to_string(),
            line: 0,
        },
        TocEntry {
            level: 4,
            title: "Child".to_string(),
            line: 5,
        },
        TocEntry {
            level: 5,
            title: "Grandchild".to_string(),
            line: 10,
        },
    ];

    let (shift, hide_root) = toc_promotion(&toc);
    assert!(!hide_root);
    assert_eq!(shift, 2);
    assert_eq!(toc_display_level(3, shift, hide_root), 1);
    assert_eq!(toc_display_level(4, shift, hide_root), 2);
    assert_eq!(toc_display_level(5, shift, hide_root), 3);
}
