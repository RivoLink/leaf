use super::common::app_helpers::{draw_app, markdown_app, status_text};
use super::common::render_helpers::line_text;
use super::lock_theme_test_state;
use crate::app::{format_comments, source_excerpt, App, Comment, COMMENT_EXPORT_HEADER};

const WIDTH: u16 = 60;
const HEIGHT: u16 = 16;
const SOURCE: &str = "# Title\n\nFirst paragraph.\n\nSecond paragraph.\n\nThird paragraph.\n";

fn app_for(source: &str) -> App {
    let mut app = markdown_app(source, usize::from(WIDTH) - 3, "doc.md", None);
    draw_app(&mut app, WIDTH, HEIGHT);
    app
}

fn rendered_idx(app: &App, needle: &str) -> usize {
    app.visible_lines(0, app.total())
        .iter()
        .position(|line| line_text(line).contains(needle))
        .unwrap_or_else(|| panic!("missing rendered line {needle:?}"))
}

fn add_comment(app: &mut App, text: &str) {
    app.begin_comment_input();
    for c in text.chars() {
        app.push_comment_char(c);
    }
    app.confirm_comment_input();
}

fn comment(start: usize, end: usize, text: &str, excerpt: &str) -> Comment {
    Comment {
        file: "doc.md".into(),
        start,
        end,
        text: text.into(),
        excerpt: excerpt.into(),
    }
}

#[test]
fn format_comments_matches_tuicr_style() {
    let items = [
        comment(3, 3, "Clarify this", "First paragraph."),
        comment(5, 7, "Merge these", ""),
    ];
    assert_eq!(
        format_comments(&items),
        format!(
            "{COMMENT_EXPORT_HEADER}\n\n\
             1. `doc.md:3` - Clarify this\n   > First paragraph.\n\
             2. `doc.md:5-7` - Merge these\n"
        )
    );
}

#[test]
fn source_excerpt_skips_blank_lines_and_truncates() {
    assert_eq!(source_excerpt("a\n\n  b  \nc", 2, 3), "b");
    assert_eq!(source_excerpt("a\n\n", 2, 3), "");
    let long = "x".repeat(100);
    let excerpt = source_excerpt(&long, 1, 1);
    assert_eq!(excerpt.chars().count(), 80);
    assert!(excerpt.ends_with("..."));
}

#[test]
fn comment_anchors_to_source_line() {
    let mut app = app_for(SOURCE);
    assert!(app.enter_comment_mode());
    app.move_comment_cursor_to(rendered_idx(&app, "Second paragraph"));
    add_comment(&mut app, "Needs work");

    let mut expected = comment(5, 5, "Needs work", "Second paragraph.");
    expected.file = "<stdin>".into();
    assert_eq!(app.comments(), &[expected]);
}

#[test]
fn range_comment_spans_source_lines() {
    let mut app = app_for(SOURCE);
    app.enter_comment_mode();
    app.move_comment_cursor_to(rendered_idx(&app, "Third paragraph"));
    app.toggle_comment_range();
    app.move_comment_cursor_to(rendered_idx(&app, "First paragraph"));
    assert_eq!(app.comment_source_label().as_deref(), Some("L3-7"));
    add_comment(&mut app, "Tighten");

    let saved = &app.comments()[0];
    assert_eq!((saved.start, saved.end), (3, 7));
    assert_eq!(saved.excerpt, "First paragraph.");
    assert!(!app.has_comment_range());
}

#[test]
fn editing_with_empty_text_deletes_comment() {
    let mut app = app_for(SOURCE);
    app.enter_comment_mode();
    app.move_comment_cursor_to(rendered_idx(&app, "First paragraph"));
    add_comment(&mut app, "One");

    app.begin_comment_input();
    assert_eq!(
        app.comment_draft().map(|d| d.comment.text.as_str()),
        Some("One")
    );
    app.clear_comment_draft_text();
    app.confirm_comment_input();
    assert!(app.comments().is_empty());
}

#[test]
fn clear_all_requires_second_press() {
    let mut app = app_for(SOURCE);
    app.enter_comment_mode();
    add_comment(&mut app, "One");
    app.clear_all_comments();
    assert_eq!(app.comments().len(), 1);
    app.clear_all_comments();
    assert!(app.comments().is_empty());
}

#[test]
fn quit_requires_confirmation_with_uncopied_comments() {
    let mut app = app_for(SOURCE);
    assert!(app.confirm_quit_with_comments());
    app.enter_comment_mode();
    add_comment(&mut app, "One");
    assert!(!app.confirm_quit_with_comments());
    assert!(app.confirm_quit_with_comments());
    app.disarm_comment_quit();
    assert!(!app.confirm_quit_with_comments());
}

#[test]
fn jump_to_comment_moves_cursor_between_comments() {
    let mut app = app_for(SOURCE);
    app.enter_comment_mode();
    let first = rendered_idx(&app, "First paragraph");
    let third = rendered_idx(&app, "Third paragraph");
    app.move_comment_cursor_to(first);
    add_comment(&mut app, "A");
    app.move_comment_cursor_to(third);
    add_comment(&mut app, "B");

    app.move_comment_cursor_to(0);
    app.jump_to_comment(true);
    assert_eq!(app.comment_cursor(), Some(first));
    app.jump_to_comment(true);
    assert_eq!(app.comment_cursor(), Some(third));
    app.jump_to_comment(false);
    assert_eq!(app.comment_cursor(), Some(first));
}

#[test]
fn render_shows_markers_and_status() {
    let _guard = lock_theme_test_state();
    let mut app = app_for(SOURCE);
    app.enter_comment_mode();
    app.move_comment_cursor_to(rendered_idx(&app, "First paragraph"));
    add_comment(&mut app, "Hover text");
    app.clear_comment_flash();

    let buffer = draw_app(&mut app, WIDTH, HEIGHT);
    let row_of = |needle: &str| {
        (0..HEIGHT)
            .find(|&y| {
                (0..WIDTH)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .contains(needle)
            })
            .unwrap()
    };
    assert_eq!(buffer[(0, row_of("First paragraph"))].symbol(), "▌");
    let status = status_text(&buffer);
    assert!(status.contains("COMMENT L3"), "{status}");
    assert!(status.contains("Hover text"), "{status}");

    app.exit_comment_mode();
    let buffer = draw_app(&mut app, WIDTH, HEIGHT);
    assert_eq!(buffer[(0, row_of("First paragraph"))].symbol(), "▎");
    assert!(status_text(&buffer).contains("1 comment"));
}

#[test]
fn comment_mode_requires_document_content() {
    let mut app = markdown_app("", 40, "", None);
    assert!(!app.enter_comment_mode());
    assert!(!app.is_comment_mode());
}

#[test]
fn code_file_comment_uses_original_line_and_excerpt() {
    let dir = std::env::temp_dir().join(format!("leaf-comments-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.rs");
    std::fs::write(&path, "fn main() {}\nlet answer = 42;\n").unwrap();

    let mut app = app_for(SOURCE);
    app.enter_comment_mode();
    assert!(app.load_path(path, &super::test_syntax_set(), super::test_theme_set()));
    assert!(!app.is_comment_mode());
    draw_app(&mut app, WIDTH, HEIGHT);

    app.enter_comment_mode();
    app.move_comment_cursor_to(rendered_idx(&app, "let answer"));
    add_comment(&mut app, "Name it");
    let saved = &app.comments()[0];
    assert_eq!(saved.start, 2);
    assert_eq!(saved.excerpt, "let answer = 42;");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn fenced_code_rows_anchor_to_their_own_source_line() {
    let source = "Intro\n\n```rust\nlet a = 1;\nlet b = 2;\n```\n";
    let mut app = app_for(source);
    app.enter_comment_mode();
    app.move_comment_cursor_to(rendered_idx(&app, "let b"));
    add_comment(&mut app, "Rename");
    let saved = &app.comments()[0];
    assert_eq!((saved.start, saved.excerpt.as_str()), (5, "let b = 2;"));
}

#[test]
fn status_bar_updates_when_one_comment_flash_replaces_another() {
    let _guard = lock_theme_test_state();
    let mut app = app_for(SOURCE);
    app.enter_comment_mode();
    add_comment(&mut app, "One");
    assert!(status_text(&draw_app(&mut app, WIDTH, HEIGHT)).contains("Comment saved"));
    app.confirm_quit_with_comments();
    assert!(status_text(&draw_app(&mut app, WIDTH, HEIGHT)).contains("press q again"));
}
