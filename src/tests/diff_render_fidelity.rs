use super::test_assets;
use crate::app::{App, DiffSpec};
use crate::diff::{parse_unified_diff, DiffLayout};
use ratatui::{backend::TestBackend, text::Line, Terminal};

fn diff_app(src: &str) -> App {
    let (ss, theme) = test_assets();
    let mut app = App::new(
        Vec::<Line<'static>>::new(),
        Vec::new(),
        "stdin".to_string(),
        false,
        false,
        None,
        None,
    );
    app.install_diff(
        parse_unified_diff(src),
        DiffSpec::default(),
        ss,
        std::sync::Arc::new(theme),
    );
    app
}

fn screen(app: &mut App, width: u16, height: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| crate::render::ui(f, app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| {
            let mut row = String::new();
            let mut x = 0;
            while x < width {
                let cell = &buffer[(x, y)];
                row.push_str(cell.symbol());
                x += crate::markdown::display_width(cell.symbol()).max(1) as u16;
            }
            row
        })
        .collect()
}

fn to_split(app: &mut App) {
    let (ss, theme) = test_assets();
    app.build_split_now(&ss, &theme);
    assert!(app.diff_toggle_layout(&ss, &theme));
    assert_eq!(app.diff().unwrap().layout, DiffLayout::Split);
}

fn right_border_col(row: &str) -> Option<usize> {
    let mut col = 0;
    let mut last = None;
    for c in row.chars() {
        if c == '│' || c == '┐' || c == '┘' {
            last = Some(col);
        }
        col += crate::markdown::display_width(&c.to_string());
    }
    last
}

const WIDE: &str = "diff --git a/j.txt b/j.txt\n--- a/j.txt\n+++ b/j.txt\n@@ -1,3 +1,3 @@\n \
                    日本語のテキストです日本語のテキストです日本語のテキストです日本語\n\
                    -古い行\n+新しい行🎉🎉🎉\n";

fn assert_aligned_right_border(rows: &[String]) {
    let box_rows: Vec<&String> = rows
        .iter()
        .filter(|r| r.trim_start().starts_with(['│', '┌', '├', '└']))
        .collect();
    let cols: Vec<Option<usize>> = box_rows.iter().map(|r| right_border_col(r)).collect();
    assert!(
        cols.iter().all(|c| c.is_some() && *c == cols[0]),
        "{box_rows:#?}"
    );
}

#[test]
fn wide_chars_keep_unified_box_aligned() {
    let mut app = diff_app(WIDE);
    let rows = screen(&mut app, 50, 9);
    assert_aligned_right_border(&rows[..8]);
}

#[test]
fn wide_chars_keep_split_columns_aligned() {
    let mut app = diff_app(WIDE);
    to_split(&mut app);
    let rows = screen(&mut app, 50, 8);
    assert_aligned_right_border(&rows[..7]);
    let body = rows.iter().find(|r| r.contains("古い行")).unwrap();
    assert!(body.contains("│  2│+ 新しい行"), "{body:?}");
}

#[test]
fn tabs_are_expanded_in_diff_lines() {
    let src =
        "diff --git a/x.go b/x.go\n--- a/x.go\n+++ b/x.go\n@@ -1 +1 @@\n-\tx := 1\n+\tx := 2\n";
    let mut app = diff_app(src);
    let rows = screen(&mut app, 50, 9);
    assert!(rows.iter().any(|r| r.contains("-     x := 1")), "{rows:#?}");
}

#[test]
fn crlf_change_is_marked_on_the_crlf_side_only() {
    let src =
        "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n-one\r\n-two\r\n+one\n+two\r\n";
    let files = parse_unified_diff(src);
    assert_eq!(
        files[0].hunks[0].lines[0],
        crate::diff::HunkLine::Del("one\r".into()),
        "the parser keeps the CR as content"
    );
    let mut app = diff_app(src);
    let rows = screen(&mut app, 50, 11);
    let one_del = rows.iter().find(|r| r.contains("- one")).unwrap();
    let one_add = rows.iter().find(|r| r.contains("+ one")).unwrap();
    let two_del = rows.iter().find(|r| r.contains("- two")).unwrap();
    assert!(one_del.contains("one␍"), "{one_del:?}");
    assert!(!one_add.contains('␍'), "{one_add:?}");
    assert!(
        !two_del.contains('␍'),
        "same ending on both sides: {two_del:?}"
    );
}

#[test]
fn word_diff_lines_render_in_split() {
    let src = "diff --git a/x.txt b/x.txt\n--- a/x.txt\n+++ b/x.txt\n@@ -1,2 +1,2 @@\n\
               foo [-bar-]{+baz+} qux\nkeep\n";
    let mut app = diff_app(src);
    to_split(&mut app);
    let rows = screen(&mut app, 60, 8);
    let row = rows.iter().find(|r| r.contains("foo")).unwrap();
    assert!(row.contains("- foo bar qux"), "{row:?}");
    assert!(row.contains("+ foo baz qux"), "{row:?}");
    assert!(rows[0].contains("+1 −1"), "{:?}", rows[0]);
}

#[test]
fn combined_lines_span_one_column_in_split() {
    let src = "diff --cc m.txt\nindex 1,2..3\n--- a/m.txt\n+++ b/m.txt\n@@@ -1,1 -1,1 +1,1 @@@\n\
               ++merged line that is quite long and should be cut at the right edge of the box\n";
    let mut app = diff_app(src);
    to_split(&mut app);
    let rows = screen(&mut app, 60, 8);
    let row = rows.iter().find(|r| r.contains("merged")).unwrap();
    assert_eq!(row.matches("merged").count(), 1, "{row:?}");
    assert!(
        row.trim_end().ends_with("…│") || row.contains("…│"),
        "{row:?}"
    );
}

#[test]
fn layout_toggle_keeps_the_same_file_in_view() {
    let mut src = String::new();
    for i in 0..20 {
        src.push_str(&format!(
            "diff --git a/f{i}.txt b/f{i}.txt\n--- a/f{i}.txt\n+++ b/f{i}.txt\n\
             @@ -1,4 +1,4 @@\n a{i}\n-old{i}\n-old2{i}\n+new{i}\n+new2{i}\n b{i}\n"
        ));
    }
    let mut app = diff_app(&src);
    let _ = screen(&mut app, 80, 6);
    assert!(app.diff_nav_to_file(12));
    to_split(&mut app);
    let state = app.diff().unwrap();
    assert_eq!(state.split_file_owner[app.scroll()], Some(12));

    let (ss, theme) = test_assets();
    assert!(app.diff_toggle_layout(&ss, &theme));
    let state = app.diff().unwrap();
    assert_eq!(state.file_owner[app.scroll()], Some(12));
}

const WORD_DIFF_MIXED: &str = "diff --git a/w.txt b/w.txt\n--- a/w.txt\n+++ b/w.txt\n\
                               @@ -1,4 +1,4 @@\nkeep1\n[-gone entirely-]\n{+brand new+}\n\
                               foo [-bar-]{+baz+} qux\nkeep2\n";

fn row_numbers(app: &App, layout: DiffLayout, needle: &str) -> (u32, u32) {
    let state = app.diff().unwrap();
    let (lines, old, new) = match layout {
        DiffLayout::Unified => (
            app.visible_lines(0, app.total()).to_vec(),
            &state.line_old_num,
            &state.line_new_num,
        ),
        DiffLayout::Split => (
            state.split_lines.clone(),
            &state.split_line_old_num_left,
            &state.split_line_new_num_right,
        ),
    };
    let row = lines
        .iter()
        .position(|l| l.spans.iter().any(|s| s.content.contains(needle)))
        .unwrap();
    (old[row], new[row])
}

#[test]
fn word_diff_numbering_matches_between_layouts() {
    let mut app = diff_app(WORD_DIFF_MIXED);
    let unified = row_numbers(&app, DiffLayout::Unified, "keep2");
    to_split(&mut app);
    let split = row_numbers(&app, DiffLayout::Split, "keep2");
    assert_eq!(unified, (4, 4));
    assert_eq!(split, unified);
}

#[test]
fn unified_word_diff_rows_get_the_gutter() {
    let mut app = diff_app(WORD_DIFF_MIXED);
    let rows = screen(&mut app, 60, 12);
    let row = rows.iter().find(|r| r.contains("qux")).unwrap();
    assert!(row.contains("│3 3│"), "{row:?}");
}

#[test]
fn split_word_diff_side_without_deletion_is_neutral() {
    let src =
        "diff --git a/w.txt b/w.txt\n--- a/w.txt\n+++ b/w.txt\n@@ -1 +1 @@\nfoo {+bar+} qux\n";
    let mut app = diff_app(src);
    to_split(&mut app);
    let state = app.diff().unwrap();
    let row = state
        .split_lines
        .iter()
        .position(|l| l.spans.iter().any(|s| s.content.contains("qux")))
        .unwrap();
    assert_eq!(state.split_line_old_num_left[row], 1);
    assert_eq!(
        state.split_line_new_num_left[row], 1,
        "left numbers like context"
    );
    assert_eq!(state.split_line_new_num_right[row], 1);
    assert_eq!(
        state.split_line_old_num_right[row], 0,
        "right side is an addition"
    );
}

#[test]
fn split_word_diff_tabs_follow_each_column() {
    let src = "diff --git a/t.txt b/t.txt\n--- a/t.txt\n+++ b/t.txt\n@@ -1 +1 @@\n\
               [-abcdef-]{+x+}\tTAB\n";
    let mut app = diff_app(src);
    to_split(&mut app);
    let line = app
        .diff()
        .unwrap()
        .split_lines
        .iter()
        .find(|l| l.spans.iter().any(|s| s.content.contains("TAB")))
        .unwrap()
        .clone();
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(text.contains("abcdef  TAB"), "{text:?}");
    assert!(text.contains("x   TAB"), "{text:?}");
}

#[test]
fn caret_notation_counts_before_tab_stops() {
    let (text, _) = crate::render::display_text("\u{1}\tA", 0);
    assert_eq!(text, "^A  A");
}

#[test]
fn crlf_marker_gets_the_intra_background_without_word_masks() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n\
               -completely old text here\r\n+something else entirely\n";
    let app = diff_app(src);
    let intra = crate::theme::app_theme().diff.intra_del;
    let row = app
        .visible_lines(0, app.total())
        .iter()
        .find(|l| l.spans.iter().any(|s| s.content.contains('␍')))
        .unwrap()
        .clone();
    let marker = row.spans.iter().find(|s| s.content.contains('␍')).unwrap();
    assert_eq!(marker.style.bg, Some(intra));
}

#[test]
fn split_merge_rows_align_with_the_gutter() {
    let src = "diff --cc m.txt\nindex 1,2..3\n--- a/m.txt\n+++ b/m.txt\n@@@ -1,1 -1,1 +1,2 @@@\n\
               - a\n -b\n++merged line that is quite long and should be cut at the right edge\n";
    let mut app = diff_app(src);
    to_split(&mut app);
    let rows = screen(&mut app, 60, 9);
    let merged = rows.iter().find(|r| r.contains("merged")).unwrap();
    assert!(merged.contains("│   │++ merged"), "{merged:?}");
    assert!(merged.contains("…│"), "{merged:?}");
}

#[test]
fn split_word_diff_empty_side_is_blank_and_neutral() {
    let src = "diff --git a/w.txt b/w.txt\n--- a/w.txt\n+++ b/w.txt\n@@ -1,2 +1,2 @@\n\
               {+new line+}\n[-gone-]\n";
    let mut app = diff_app(src);
    to_split(&mut app);
    let state = app.diff().unwrap();
    let row_of = |needle: &str| {
        state
            .split_lines
            .iter()
            .position(|l| l.spans.iter().any(|s| s.content.contains(needle)))
            .unwrap()
    };
    let added = row_of("new line");
    assert_eq!(
        (
            state.split_line_old_num_left[added],
            state.split_line_new_num_left[added]
        ),
        (0, 0)
    );
    let gone = row_of("gone");
    assert_eq!(
        (
            state.split_line_old_num_right[gone],
            state.split_line_new_num_right[gone]
        ),
        (0, 0)
    );
}

#[test]
fn unified_word_diff_separator_shares_the_sigil_background() {
    let src = "diff --git a/w.txt b/w.txt\n--- a/w.txt\n+++ b/w.txt\n@@ -0,0 +1 @@\n{+x+}\n";
    let app = diff_app(src);
    let add_bg = crate::theme::app_theme().diff.add_bg;
    let row = app
        .visible_lines(0, app.total())
        .iter()
        .find(|l| l.spans.iter().any(|s| s.content == "+"))
        .unwrap()
        .clone();
    assert_eq!(row.spans[2].content, " ");
    assert_eq!(row.spans[2].style.bg, Some(add_bg));
}

#[test]
fn split_merge_row_fill_keeps_its_background_with_gutter() {
    let src = "diff --cc m.txt\nindex 1,2..3\n--- a/m.txt\n+++ b/m.txt\n@@@ -1,1 -1,1 +1,2 @@@\n\
               ++ short\n";
    let mut app = diff_app(src);
    to_split(&mut app);
    let mut terminal = Terminal::new(TestBackend::new(60, 9)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let add_bg = crate::theme::app_theme().diff.add_bg;
    let y = (0..9)
        .find(|&y| (0..60).any(|x| buffer[(x, y)].symbol() == "h"))
        .unwrap();
    let x_text_end = (0..60)
        .rev()
        .find(|&x| buffer[(x, y)].symbol() == "t")
        .unwrap();
    assert_eq!(
        buffer[(x_text_end + 3, y)].bg,
        add_bg,
        "the fill continues after the text"
    );
}

fn fill_bg_after(app: &mut App, needle: &str, last: &str) -> ratatui::style::Color {
    let mut terminal = Terminal::new(TestBackend::new(70, 20)).unwrap();
    terminal.draw(|f| crate::render::ui(f, app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let row_text = |y: u16| (0..70).map(|x| buffer[(x, y)].symbol()).collect::<String>();
    let y = (0..20).find(|&y| row_text(y).contains(needle)).unwrap();
    let x_end = (0..70)
        .rev()
        .find(|&x| buffer[(x, y)].symbol() == last)
        .unwrap();
    buffer[(x_end + 3, y)].bg
}

#[test]
fn unified_mixed_word_diff_row_fill_has_no_change_background() {
    let src = "diff --git a/w.txt b/w.txt\n--- a/w.txt\n+++ b/w.txt\n@@ -1 +1 @@\n\
               foo [-bar-]{+baz+} qux\n";
    let mut app = diff_app(src);
    let diff = crate::theme::app_theme().diff;
    let bg = fill_bg_after(&mut app, "qux", "x");
    assert_ne!(bg, diff.del_bg);
    assert_ne!(bg, diff.add_bg);
}

#[test]
fn active_search_row_is_not_filled_to_the_border_in_either_layout() {
    let src = "diff --git a/s.txt b/s.txt\n--- a/s.txt\n+++ b/s.txt\n@@ -1,2 +1,2 @@\n \
               keepme\n-old\n+new\n";
    let highlight = crate::theme::app_theme().markdown.search_highlight_bg;
    for numbers in [true, false] {
        let mut app = diff_app(src);
        app.set_line_numbers_visible(numbers);
        app.set_search_query("keepme");
        app.run_search();
        app.scroll = 0;
        assert_ne!(
            fill_bg_after(&mut app, "keepme", "e"),
            highlight,
            "{numbers}"
        );
        to_split(&mut app);
        assert_ne!(
            fill_bg_after(&mut app, "keepme", "e"),
            highlight,
            "{numbers}"
        );
    }
}

#[test]
fn active_changed_search_row_keeps_its_change_fill() {
    let src = "diff --git a/s.txt b/s.txt\n--- a/s.txt\n+++ b/s.txt\n@@ -1,2 +1,2 @@\n \
               ctx\n-old\n+newmark\n";
    let mut app = diff_app(src);
    app.set_line_numbers_visible(false);
    app.set_search_query("newmark");
    app.run_search();
    app.scroll = 0;
    let add_bg = crate::theme::app_theme().diff.add_bg;
    assert_eq!(fill_bg_after(&mut app, "newmark", "k"), add_bg);
}

#[test]
fn active_merge_search_row_keeps_its_fill() {
    let src = "diff --cc m.txt\nindex 1,2..3\n--- a/m.txt\n+++ b/m.txt\n@@@ -1,1 -1,1 +1,2 @@@\n\
               ++ mergemark\n";
    let add_bg = crate::theme::app_theme().diff.add_bg;
    for numbers in [false, true] {
        let mut app = diff_app(src);
        app.set_line_numbers_visible(numbers);
        app.set_search_query("mergemark");
        app.run_search();
        app.scroll = 0;
        if !numbers {
            assert_eq!(fill_bg_after(&mut app, "mergemark", "k"), add_bg);
        }
        to_split(&mut app);
        assert_eq!(
            fill_bg_after(&mut app, "mergemark", "k"),
            add_bg,
            "{numbers}"
        );
    }
}

#[test]
fn active_split_row_with_an_empty_side_highlights_only_its_text_side() {
    let src = "diff --git a/s.txt b/s.txt\n--- a/s.txt\n+++ b/s.txt\n@@ -1,2 +1 @@\n \
               ctx\n-gonemark\n";
    let highlight = crate::theme::app_theme().markdown.search_highlight_bg;
    for numbers in [false, true] {
        let mut app = diff_app(src);
        app.set_line_numbers_visible(numbers);
        app.set_search_query("gonemark");
        app.run_search();
        app.scroll = 0;
        to_split(&mut app);
        let mut terminal = Terminal::new(TestBackend::new(70, 20)).unwrap();
        terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let row_text = |y: u16| (0..70).map(|x| buffer[(x, y)].symbol()).collect::<String>();
        let y = (0..20).find(|&y| row_text(y).contains("gonemark")).unwrap();
        let middle = (1..69)
            .filter(|&x| buffer[(x, y)].symbol() == "│")
            .nth(if numbers { 2 } else { 1 })
            .unwrap();
        let lit: Vec<u16> = (middle + 1..70)
            .filter(|&x| buffer[(x, y)].bg == highlight)
            .collect();
        assert!(
            lit.is_empty(),
            "numbers={numbers} highlighted cells {lit:?}"
        );
    }
}
