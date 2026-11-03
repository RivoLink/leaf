use crate::{
    app::{App, DiffSpec},
    config::PageScrollSize,
    diff::parse_unified_diff,
};
use ratatui::{layout::Rect, text::Line};

fn make_app(viewport_height: u16) -> App {
    let lines = (0..100)
        .map(|idx| Line::from(format!("line {idx}")))
        .collect();
    let mut app = App::new(
        lines,
        Vec::new(),
        "test".to_string(),
        false,
        false,
        None,
        None,
    );
    app.content_area = Rect::new(0, 0, 80, viewport_height);
    app
}

#[test]
fn page_scroll_defaults_to_twenty_rows() {
    let mut app = make_app(10);
    app.scroll_page_down();
    assert_eq!(app.scroll(), 20);
}

#[test]
fn page_scroll_percent_uses_ceil_and_allows_more_than_one_page() {
    let mut app = make_app(10);
    app.set_page_scroll_size(PageScrollSize::Percent(1));
    app.scroll_page_down();
    assert_eq!(app.scroll(), 1);

    app.set_page_scroll_size(PageScrollSize::Percent(150));
    app.scroll_page_down();
    assert_eq!(app.scroll(), 16);
}

#[test]
fn page_scroll_keep_rows_overlaps_symmetrically() {
    let mut app = make_app(10);
    app.set_page_scroll_size(PageScrollSize::KeepRows(3));
    app.scroll_page_down();
    assert_eq!(app.scroll(), 7);

    app.scroll_page_up();
    assert_eq!(app.scroll(), 0);

    app.scroll_to(20);
    app.scroll_page_up();
    assert_eq!(app.scroll(), 13);
}

#[test]
fn page_scroll_clamps_large_steps_without_overflow() {
    let mut app = make_app(10);
    app.set_page_scroll_size(PageScrollSize::Rows(usize::MAX));
    app.scroll_page_down();
    assert_eq!(app.scroll(), app.max_scroll());
}

#[test]
fn page_scroll_percent_uses_diff_preview_body_capacity() {
    let mut diff =
        String::from("diff --git a/file b/file\n--- a/file\n+++ b/file\n@@ -1,20 +1,20 @@\n");
    for idx in 0..20 {
        diff.push_str(&format!("-before {idx}\n+after {idx}\n"));
    }
    let files = parse_unified_diff(&diff);
    let mut app = make_app(10);
    let (ss, theme) = super::test_assets();
    app.install_diff(
        files,
        DiffSpec::default(),
        ss.clone(),
        std::sync::Arc::new(theme.clone()),
    );
    assert!(app.diff_toggle_preview(&ss, &theme));
    app.set_page_scroll_size(PageScrollSize::Percent(100));

    app.scroll_page_down();
    assert_eq!(app.scroll(), 8);
}
