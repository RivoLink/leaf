use super::super::test_assets;
use crate::render::FrameDecorations;

pub(crate) fn plain(lines: &[ratatui::text::Line<'_>]) -> Vec<String> {
    lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        })
        .collect()
}

pub(crate) fn line_text(line: &ratatui::text::Line<'_>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

pub(crate) fn diff_theme_for_test() -> crate::theme::DiffTheme {
    crate::theme::app_theme().diff
}

pub(crate) fn last_char(line: &ratatui::text::Line<'_>) -> Option<char> {
    line.spans.iter().flat_map(|s| s.content.chars()).last()
}

pub(crate) fn line_width_chars(line: &ratatui::text::Line<'_>) -> usize {
    line.spans.iter().map(|s| s.content.chars().count()).sum()
}

pub(crate) fn build_gutter_lines_for_test() -> (
    crate::theme::DiffTheme,
    Vec<ratatui::text::Line<'static>>,
    crate::app::DiffState,
) {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,2 +1,3 @@
 keep
-old
+new
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let mut state = crate::app::DiffState::new(std::sync::Arc::new(files), Default::default());
    state.hunk_ranges = built.hunk_ranges.clone();
    state.line_owner = built.line_owner.clone();
    state.line_old_num = built.line_old_num.clone();
    state.line_new_num = built.line_new_num.clone();
    state.file_header_status = built.file_header_status.clone();
    let mut lines = built.lines.clone();
    let diff_theme = diff_theme_for_test();
    crate::render::apply_number_gutter(&mut lines, &state, 0, &diff_theme);
    (diff_theme, lines, state)
}

pub(crate) fn find_gutter_line_for<F>(
    lines: &[ratatui::text::Line<'static>],
    state: &crate::app::DiffState,
    pred: F,
) -> Vec<ratatui::text::Span<'static>>
where
    F: Fn(u32, u32) -> bool,
{
    for (i, line) in lines.iter().enumerate() {
        let first = line.spans.first();

        let is_gutter_body = first.is_some_and(|s| s.content == "│") && line.spans.len() >= 7;
        if !is_gutter_body {
            continue;
        }
        let o = state.line_old_num.get(i).copied().unwrap_or(0);
        let n = state.line_new_num.get(i).copied().unwrap_or(0);
        if pred(o, n) {
            return line.spans.clone();
        }
    }
    panic!("no matching body line found");
}

pub(crate) fn build_mixed_hunk_header_line() -> ratatui::text::Line<'static> {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -3,2 +4,2 @@
 keep
-old
+new
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    built
        .lines
        .into_iter()
        .find(|l| line_text(l).starts_with("├─ around line "))
        .expect("hunk header row present")
}

pub(crate) fn build_two_file_diff_state(
) -> (crate::app::DiffState, Vec<ratatui::text::Line<'static>>) {
    let (ss, theme) = test_assets();
    let src = "\
diff --git a/a b/a
--- a/a
+++ b/a
@@ -1,1 +1,3 @@
 keep
+one
+two
diff --git a/b b/b
--- a/b
+++ b/b
@@ -1,1 +1,2 @@
 keep
+alpha
";
    let files = crate::diff::parse_unified_diff(src);
    let built = crate::render::build_diff_lines(&files, &ss, &theme);
    let mut state = crate::app::DiffState::new(std::sync::Arc::new(files), Default::default());
    state.hunk_ranges = built.hunk_ranges.clone();
    state.line_owner = built.line_owner.clone();
    state.line_old_num = built.line_old_num.clone();
    state.line_new_num = built.line_new_num.clone();
    state.file_header_status = built.file_header_status.clone();
    state.file_owner = built.file_owner.clone();
    state.file_top_idx = built.file_top_idx.clone();
    state.file_bottom_idx = built.file_bottom_idx.clone();
    (state, built.lines)
}

pub(crate) fn body_content_char_bgs(
    line: &ratatui::text::Line<'_>,
) -> Vec<(char, Option<ratatui::style::Color>)> {
    let mut out = Vec::new();
    for span in line.spans.iter().skip(3) {
        for ch in span.content.chars() {
            out.push((ch, span.style.bg));
        }
    }
    out
}

#[cfg(test)]
pub(crate) fn render_split_at_width(width: u16, line_numbers: bool) -> ratatui::buffer::Buffer {
    use crate::app::{App, DiffSpec};
    use ratatui::{backend::TestBackend, Terminal};
    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,2 +1,2 @@
-old
+new
 keep
";
    let files = crate::diff::parse_unified_diff(src);
    let mut app = App::new(
        vec![],
        vec![],
        "stdin".to_string(),
        false,
        false,
        None,
        None,
    );
    let (ss, theme) = test_assets();
    app.set_source_for_test(src.to_string());
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    app.build_split_now(&ss, &theme);
    app.diff_mut().unwrap().layout = crate::diff::DiffLayout::Split;
    app.set_line_numbers_visible(line_numbers);
    let mut terminal = Terminal::new(TestBackend::new(width, 12)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    terminal.backend().buffer().clone()
}

#[cfg(test)]
pub(crate) fn middle_divider_col(buffer: &ratatui::buffer::Buffer, row: u16) -> Option<u16> {
    let w = buffer.area.width;
    if w < 4 {
        return None;
    }
    let mid = 1u16 + (w - 3) / 2;
    if buffer.cell((mid, row)).unwrap().symbol() == "│" {
        Some(mid)
    } else {
        None
    }
}

#[cfg(test)]
pub(crate) fn left_gutter_col_split(buffer: &ratatui::buffer::Buffer) -> Option<u16> {
    let w = buffer.area.width;
    if w < 4 {
        return None;
    }
    let mid = 1u16 + (w - 3) / 2;
    for y in 0..buffer.area.height {
        if buffer.cell((1u16, y)).unwrap().symbol() != "│" {
            continue;
        }

        if buffer.cell((mid, y)).unwrap().symbol() != "│" {
            continue;
        }

        for x in 2..mid {
            if buffer.cell((x, y)).unwrap().symbol() == "│" {
                return Some(x);
            }
        }
    }
    None
}

#[cfg(test)]
pub(crate) fn render_split_long_lines(width: u16) -> ratatui::buffer::Buffer {
    use crate::app::{App, DiffSpec};
    use ratatui::{backend::TestBackend, Terminal};
    let long_del: String = "D".repeat(300);
    let long_add: String = "A".repeat(300);
    let src = format!(
        "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n-{long_del}\n+{long_add}\n keep\n"
    );
    let files = crate::diff::parse_unified_diff(&src);
    let mut app = App::new(
        vec![],
        vec![],
        "stdin".to_string(),
        false,
        false,
        None,
        None,
    );
    let (ss, theme) = test_assets();
    app.set_source_for_test(src.clone());
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    app.build_split_now(&ss, &theme);
    app.diff_mut().unwrap().layout = crate::diff::DiffLayout::Split;
    app.set_line_numbers_visible(false);
    let mut terminal = Terminal::new(TestBackend::new(width, 12)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    terminal.backend().buffer().clone()
}

#[cfg(test)]
pub(crate) fn first_blank_spacer_row(buffer: &ratatui::buffer::Buffer) -> Option<u16> {
    for y in 0..buffer.area.height {
        let mut has_border_left = false;
        let mut has_frame = false;
        for x in 0..buffer.area.width {
            let sym = buffer.cell((x, y)).unwrap().symbol();
            if sym == "│" {
                has_border_left = true;
            }
            if sym == "┌" || sym == "├" || sym == "└" || sym == "┐" || sym == "┘" {
                has_frame = true;
            }
        }
        if !has_border_left || has_frame {
            continue;
        }

        let mut has_content = false;
        for x in 0..buffer.area.width {
            let sym = buffer.cell((x, y)).unwrap().symbol();
            if sym == "-" || sym == "+" || sym.chars().any(|c| c.is_ascii_alphanumeric()) {
                has_content = true;
                break;
            }
        }
        if has_content {
            continue;
        }
        return Some(y);
    }
    None
}

#[cfg(test)]
pub(crate) fn render_split_empty_context(width: u16) -> ratatui::buffer::Buffer {
    use crate::app::{App, DiffSpec};
    use ratatui::{backend::TestBackend, Terminal};

    let src = "\
diff --git a/x b/x
--- a/x
+++ b/x
@@ -1,4 +1,5 @@
 esac

 if [[
+added
 tail
";
    let files = crate::diff::parse_unified_diff(src);
    let mut app = App::new(
        vec![],
        vec![],
        "stdin".to_string(),
        false,
        false,
        None,
        None,
    );
    let (ss, theme) = test_assets();
    app.set_source_for_test(src.to_string());
    app.install_diff(
        files,
        DiffSpec::default(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    app.build_split_now(&ss, &theme);
    app.diff_mut().unwrap().layout = crate::diff::DiffLayout::Split;
    app.set_line_numbers_visible(true);
    let mut terminal = Terminal::new(TestBackend::new(width, 20)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    terminal.backend().buffer().clone()
}

#[cfg(test)]
pub(crate) fn render_unified_row_cells(
    sigil: char,
    bg: ratatui::style::Color,
    content: &str,
    width: u16,
) -> Vec<(String, ratatui::style::Color)> {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    let theme = diff_theme_for_test();
    let sigil_style = Style::default().bg(bg);
    let sep_style = Style::default().bg(bg);
    let content_style = Style::default().bg(bg);
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled("│".to_string(), Style::default().fg(theme.frame_fg)),
        Span::styled(sigil.to_string(), sigil_style),
        Span::styled(" ".to_string(), sep_style),
        Span::styled(content.to_string(), content_style),
    ];
    crate::render::truncate_unified_body_row(&mut spans, width as usize, Some(bg));
    let mut lines = vec![Line::from(spans)];
    crate::render::close_frame_right(
        &mut lines,
        width,
        &theme,
        &FrameDecorations {
            auto_tee_at_gutter_col: true,
            ..Default::default()
        },
    );

    let mut cells: Vec<(String, ratatui::style::Color)> = Vec::new();
    for span in &lines[0].spans {
        let cell_bg = span.style.bg.unwrap_or(ratatui::style::Color::Reset);
        for ch in span.content.chars() {
            cells.push((ch.to_string(), cell_bg));
        }
    }
    cells
}
