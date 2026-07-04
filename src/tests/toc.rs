use super::{test_assets, test_md_theme};
use crate::app::{App, TocScrollMode};
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
    let (lines, _, _, _) = parse_markdown(&md, &ss, &theme, &test_md_theme(), false, true).into();
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
        true,
    )
    .into();

    assert_eq!(toc.len(), 3);
    assert_eq!(toc[0].level, 1);
    assert_eq!(toc[1].level, 2);
    assert_eq!(toc[2].level, 3);
}

#[test]
fn frontmatter_is_ignored_in_toc() {
    let (ss, theme) = test_assets();
    let src = "---\ntitle: Demo\nowner: me\n---\n# Visible\nBody\n";
    let (_, toc, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();

    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].title, "Visible");
}

#[test]
fn toc_hides_unique_top_and_promotes_when_shallow() {
    let toc = toc(&[(1, 0), (2, 10), (2, 20)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, None);
    assert_eq!(levels.display_level(1), None);
    assert_eq!(levels.display_level(2), Some(1));
}

#[test]
fn toc_hides_unique_top_and_shows_two_paliers() {
    let toc = toc(&[(1, 0), (2, 10), (3, 15)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, Some(3));
    assert_eq!(levels.display_level(1), None);
    assert_eq!(levels.display_level(2), Some(1));
    assert_eq!(levels.display_level(3), Some(2));
}

#[test]
fn toc_keeps_single_heading_as_root() {
    let toc = toc(&[(1, 0)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 1);
    assert_eq!(levels.sub, None);
    assert_eq!(levels.display_level(1), Some(1));
}

#[test]
fn toc_keeps_non_unique_top_as_root() {
    let toc = toc(&[(2, 0), (2, 10), (3, 14)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, Some(3));
}

#[test]
fn toc_promotes_unique_deep_root() {
    let toc = toc(&[(3, 0), (4, 5), (5, 10)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 4);
    assert_eq!(levels.sub, Some(5));
    assert_eq!(levels.display_level(3), None);
    assert_eq!(levels.display_level(4), Some(1));
    assert_eq!(levels.display_level(5), Some(2));
}

#[test]
fn toc_deep_non_unique_top_is_root() {
    let toc = toc(&[(3, 0), (3, 10), (4, 14)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 3);
    assert_eq!(levels.sub, Some(4));
}

#[test]
fn toc_promotion_is_not_recursive() {
    let toc = toc(&[(1, 0), (2, 5), (3, 8), (3, 12)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, Some(3));
    assert_eq!(levels.display_level(1), None);
    assert_eq!(levels.display_level(2), Some(1));
    assert_eq!(levels.display_level(3), Some(2));
}

#[test]
fn toc_ignores_level_gaps_two_paliers() {
    let toc = toc(&[(1, 0), (3, 5), (3, 10)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 3);
    assert_eq!(levels.sub, None);
    assert_eq!(levels.display_level(1), None);
    assert_eq!(levels.display_level(3), Some(1));
}

#[test]
fn toc_ignores_level_gaps_three_paliers() {
    let toc = toc(&[(1, 0), (2, 5), (2, 9), (4, 12)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, Some(4));
    assert_eq!(levels.display_level(2), Some(1));
    assert_eq!(levels.display_level(4), Some(2));
}

#[test]
fn toc_sub_is_next_present_palier() {
    let toc = toc(&[(2, 0), (2, 5), (4, 9)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, Some(4));
}

#[test]
fn toc_levels_empty_returns_none() {
    assert!(toc_levels(&[]).is_none());
}

#[test]
fn normalize_keeps_top_three_paliers() {
    let toc = toc(&[(2, 0), (3, 5), (4, 10), (5, 15)]);
    let normalized = normalize_toc(toc);
    assert_eq!(
        normalized.iter().map(|e| e.level).collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
}

fn make_app_with_overflowing_toc() -> App {
    let entries: Vec<(u8, usize)> = (0..20).map(|i| (2, i * 3)).collect();
    let mut app = make_app_with_toc(100, 15, toc(&entries));
    // Simulate a rendered TOC panel smaller than the entry count.
    app.toc_list_area = Some(Rect::new(0, 0, 30, 10));
    app.refresh_toc_cache();
    app
}

#[test]
fn toc_scroll_mode_default_is_auto() {
    let app = make_app_with_toc(10, 15, toc(&[(2, 0)]));
    assert_eq!(app.toc_scroll_mode(), TocScrollMode::Auto);
    assert!(!app.is_toc_scroll_hint_dismissed());
}

#[test]
fn toc_overflows_true_when_entries_exceed_list_height() {
    let app = make_app_with_overflowing_toc();
    assert!(app.toc_overflows(10));
    assert!(!app.toc_overflows(50));
}

#[test]
fn scroll_toc_down_switches_to_manual_mode_and_dismisses_hint() {
    let mut app = make_app_with_overflowing_toc();
    app.toggle_toc();
    app.scroll_toc_down(3);
    assert_eq!(app.toc_scroll_mode(), TocScrollMode::Manual(3));
    assert!(app.is_toc_scroll_hint_dismissed());
    assert_eq!(app.hovered_toc_idx, None);
}

#[test]
fn scroll_toc_does_not_dismiss_hint_when_warning_not_visible() {
    let mut app = make_app_with_overflowing_toc();
    app.scroll_toc_down(3);
    assert_eq!(app.toc_scroll_mode(), TocScrollMode::Manual(3));
    assert!(!app.is_toc_scroll_hint_dismissed());
}

#[test]
fn scroll_toc_up_saturates_at_zero() {
    let mut app = make_app_with_overflowing_toc();
    app.scroll_toc_up(5);
    assert_eq!(app.toc_scroll_mode(), TocScrollMode::Manual(0));
}

#[test]
fn scroll_toc_down_bounds_to_max_offset() {
    let mut app = make_app_with_overflowing_toc();
    let total = app.toc_display_lines().len();
    app.scroll_toc_down(1000);
    let max_offset = (total + 1).saturating_sub(10);
    assert_eq!(app.toc_scroll_mode(), TocScrollMode::Manual(max_offset));
}

#[test]
fn scroll_toc_down_then_up_composes() {
    let mut app = make_app_with_overflowing_toc();
    app.scroll_toc_down(5);
    app.scroll_toc_up(2);
    assert_eq!(app.toc_scroll_mode(), TocScrollMode::Manual(3));
}

#[test]
fn content_scroll_resets_toc_scroll_mode_to_auto() {
    let mut app = make_app_with_overflowing_toc();
    app.scroll_toc_down(5);
    assert!(matches!(app.toc_scroll_mode(), TocScrollMode::Manual(_)));
    app.scroll_down(1);
    assert_eq!(app.toc_scroll_mode(), TocScrollMode::Auto);
}

#[test]
fn scroll_to_toc_display_line_preserves_manual_offset() {
    let mut app = make_app_with_overflowing_toc();
    app.scroll_toc_down(5);
    app.scroll_to_toc_display_line(2);
    assert_eq!(app.toc_scroll_mode(), TocScrollMode::Manual(5));
}

#[test]
fn click_on_toc_entry_focuses_it_even_when_scroll_clamped_to_max() {
    let mut app = make_app_with_toc(50, 15, toc(&[(2, 0), (2, 10), (2, 44), (2, 48)]));
    app.scroll_to_toc_display_line(2);
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(2));
}

#[test]
fn content_scroll_clears_toc_click_pin() {
    let mut app = make_app_with_toc(50, 15, toc(&[(2, 0), (2, 10), (2, 44), (2, 48)]));
    app.scroll_to_toc_display_line(2);
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(2));
    app.scroll_bottom();
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(3));
}

#[test]
fn hint_hidden_when_dismissed() {
    let mut app = make_app_with_overflowing_toc();
    app.toggle_toc();
    assert!(app.is_toc_visible());
    assert!(app.is_toc_scroll_hint_visible());
    app.scroll_toc_down(1);
    assert!(!app.is_toc_scroll_hint_visible());
}

#[test]
fn hint_hidden_when_toc_does_not_overflow() {
    let mut app = make_app_with_toc(10, 15, toc(&[(2, 0), (2, 5)]));
    app.toc_list_area = Some(Rect::new(0, 0, 30, 10));
    app.refresh_toc_cache();
    app.toggle_toc();
    assert!(app.is_toc_visible());
    assert!(!app.is_toc_scroll_hint_visible());
}

#[test]
fn hint_hidden_when_toc_not_visible() {
    let app = make_app_with_overflowing_toc();
    assert!(!app.is_toc_visible());
    assert!(!app.is_toc_scroll_hint_visible());
}

#[test]
fn toc_scroll_offset_auto_keeps_active_visible() {
    let mut app = make_app_with_overflowing_toc();
    app.scroll_bottom();
    app.refresh_toc_cache();
    let offset = app.toc_scroll_offset(10);
    let total = app.toc_display_lines().len();
    let active_display_idx = app.toc_active_display_idx.unwrap();
    assert!(active_display_idx < offset + 10);
    assert!(offset <= (total + 1).saturating_sub(10));
}

#[test]
fn toc_scroll_offset_auto_reserves_padding_row_when_active_is_last() {
    let mut app = make_app_with_overflowing_toc();
    app.scroll_bottom();
    app.refresh_toc_cache();
    let list_height = 10;
    let offset = app.toc_scroll_offset(list_height);
    let total = app.toc_display_lines().len();
    assert!(offset + list_height as usize > total);
}

#[test]
fn toc_scroll_offset_manual_returns_stored_offset_bounded() {
    let mut app = make_app_with_overflowing_toc();
    app.scroll_toc_down(4);
    assert_eq!(app.toc_scroll_offset(10), 4);
    // Simulate terminal resize: with a taller window, offset should clamp.
    let total = app.toc_display_lines().len();
    assert!(app.toc_scroll_offset(1000) <= total);
}

fn make_app_with_top_and_sub_toc() -> App {
    let entries = vec![
        (1u8, 0usize),
        (2, 2),
        (2, 4),
        (1, 6),
        (2, 8),
        (1, 10),
        (2, 12),
        (2, 14),
        (1, 16),
        (2, 18),
    ];
    let mut app = make_app_with_toc(50, 15, toc(&entries));
    app.toc_list_area = Some(Rect::new(0, 0, 30, 10));
    app.toggle_toc();
    app.refresh_toc_cache();
    app
}

#[test]
fn focus_next_top_level_cycles_within_visible() {
    let mut app = make_app_with_top_and_sub_toc();
    app.scroll_to_toc_display_line(0);
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(0));

    app.focus_next_top_level_toc();
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(3));

    app.focus_next_top_level_toc();
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(5));
}

#[test]
fn focus_next_top_level_wraps_around_visible() {
    let mut app = make_app_with_top_and_sub_toc();
    app.scroll_to_toc_display_line(8);
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(8));

    app.focus_next_top_level_toc();
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(0));
}

#[test]
fn focus_prev_top_level_wraps_around_visible() {
    let mut app = make_app_with_top_and_sub_toc();
    app.scroll_to_toc_display_line(0);
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(0));

    app.focus_prev_top_level_toc();
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(8));
}

#[test]
fn focus_from_sub_entry_uses_top_level_ancestor() {
    let mut app = make_app_with_top_and_sub_toc();
    app.scroll_to_toc_display_line(4);
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(4));

    app.focus_next_top_level_toc();
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(5));
}

fn make_app_with_overflowing_top_sub_toc() -> App {
    let entries = vec![
        (1u8, 0usize),
        (2, 2),
        (1, 4),
        (2, 6),
        (1, 8),
        (2, 10),
        (1, 12),
        (2, 14),
        (1, 16),
        (2, 18),
    ];
    let mut app = make_app_with_toc(50, 15, toc(&entries));
    app.toc_list_area = Some(Rect::new(0, 0, 30, 4));
    app.toggle_toc();
    app.refresh_toc_cache();
    app
}

#[test]
fn focus_next_when_active_out_of_window_jumps_to_first_visible() {
    let mut app = make_app_with_overflowing_top_sub_toc();
    app.scroll_to_toc_display_line(0);
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(0));
    app.scroll_toc_down(6);
    app.refresh_toc_cache();
    let offset = app.toc_scroll_offset(4);
    assert!(offset > 0);

    app.focus_next_top_level_toc();
    app.refresh_toc_cache();
    let target = app.toc_active_idx.unwrap();
    let target_display_idx = app
        .toc_display_entries()
        .iter()
        .position(|&i| i == target)
        .unwrap();
    assert!(target_display_idx >= offset);
    assert!(target_display_idx < offset + 4);
}

#[test]
fn focus_prev_when_active_out_of_window_jumps_to_last_visible() {
    let mut app = make_app_with_overflowing_top_sub_toc();
    app.scroll_to_toc_display_line(0);
    app.refresh_toc_cache();
    app.scroll_toc_down(6);
    app.refresh_toc_cache();
    let offset = app.toc_scroll_offset(4);

    app.focus_prev_top_level_toc();
    app.refresh_toc_cache();
    let target = app.toc_active_idx.unwrap();
    let target_display_idx = app
        .toc_display_entries()
        .iter()
        .position(|&i| i == target)
        .unwrap();
    assert!(target_display_idx >= offset);
    assert!(target_display_idx < offset + 4);
}

#[test]
fn focus_top_level_preserves_toc_scroll_offset() {
    let mut app = make_app_with_overflowing_top_sub_toc();
    app.scroll_toc_down(2);
    let before = app.toc_scroll_offset(4);
    assert_eq!(before, 2);
    app.focus_next_top_level_toc();
    let after = app.toc_scroll_offset(4);
    assert_eq!(after, before);
}

#[test]
fn focus_top_level_noop_when_no_level_1_visible() {
    let mut app = make_app_with_toc(
        50,
        15,
        toc(&[
            (1, 0),
            (2, 2),
            (2, 4),
            (2, 6),
            (2, 8),
            (2, 10),
            (1, 12),
            (2, 14),
            (2, 16),
            (2, 18),
        ]),
    );
    app.toc_list_area = Some(Rect::new(0, 0, 30, 3));
    app.toggle_toc();
    app.refresh_toc_cache();
    app.scroll_toc_down(2);
    app.refresh_toc_cache();
    let scroll_before = app.scroll();
    let mode_before = app.toc_scroll_mode();

    app.focus_next_top_level_toc();
    assert_eq!(app.scroll(), scroll_before);
    assert_eq!(app.toc_scroll_mode(), mode_before);
}

#[test]
fn focus_top_level_ignores_reverse_mode() {
    let mut app = make_app_with_top_and_sub_toc();
    app.toggle_reverse_mode();
    app.scroll_to_toc_display_line(0);
    app.refresh_toc_cache();
    app.focus_next_top_level_toc();
    app.refresh_toc_cache();
    assert_eq!(app.toc_active_idx, Some(3));
}

#[test]
fn focus_top_level_dismisses_hint_when_visible() {
    let mut app = make_app_with_overflowing_top_sub_toc();
    assert!(app.is_toc_scroll_hint_visible());
    app.focus_next_top_level_toc();
    assert!(!app.is_toc_scroll_hint_visible());
    assert!(app.is_toc_scroll_hint_dismissed());
}
