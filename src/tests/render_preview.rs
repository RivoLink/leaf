use super::test_assets;

#[test]
fn diff_preview_panel_renders_two_columns_with_divider() {
    use crate::app::{App, DiffSpec};
    use crate::diff::parse_unified_diff;
    use ratatui::{backend::TestBackend, Terminal};

    let src = "diff --git a/README.md b/README.md\n--- a/README.md\n+++ b/README.md\n@@ -1 +1 @@\n-# Old\n+# New\n";
    let files = parse_unified_diff(src);
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

    let flipped = app.diff_toggle_preview(&ss, &theme);
    assert!(flipped);
    assert!(app.diff().unwrap().preview_visible);

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let mut header_row = String::new();
    for x in 0..buffer.area.width {
        header_row.push_str(buffer.cell((x, 0)).unwrap().symbol());
    }
    assert!(
        header_row.contains('┌') && header_row.contains('┐'),
        "preview header must have `┌…┐` corners: {header_row:?}"
    );
    assert!(
        header_row.contains('┬'),
        "preview header must have a `┬` connector for the middle divider: {header_row:?}"
    );
    assert!(
        header_row.contains("README.md"),
        "header row missing file path: {header_row:?}"
    );
    assert!(
        header_row.contains("modified"),
        "header row missing `modified` status: {header_row:?}"
    );

    let center = buffer.area.width as usize / 2;
    let scan_lo = center.saturating_sub(2);
    let scan_hi = (center + 2).min(buffer.area.width as usize - 1);
    let mut best_run = 0usize;
    for x in scan_lo..=scan_hi {
        let mut run = 0usize;
        for y in 1..buffer.area.height {
            if buffer.cell((x as u16, y)).unwrap().symbol() == "│" {
                run += 1;
            }
        }
        best_run = best_run.max(run);
    }
    assert!(
        best_run >= 1,
        "split-style preview must render a `│` mid-column rail (best run = {best_run})"
    );
}

fn render_preview_for_diff(src: &str) -> ratatui::buffer::Buffer {
    use crate::app::{App, DiffSpec};
    use crate::diff::parse_unified_diff;
    use ratatui::{backend::TestBackend, Terminal};

    let files = parse_unified_diff(src);
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
    let flipped = app.diff_toggle_preview(&ss, &theme);
    assert!(flipped, "preview must toggle on");

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    terminal.backend().buffer().clone()
}

fn row_text(buffer: &ratatui::buffer::Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
        .collect()
}

#[test]
fn preview_binary_single_column() {
    let src = "diff --git a/img.png b/img.png\nindex 1111111..2222222 100644\nBinary files a/img.png and b/img.png differ\n";
    let buffer = render_preview_for_diff(src);

    let mut found_note = false;
    for y in 0..buffer.area.height {
        if row_text(&buffer, y).contains("binary content not shown") {
            found_note = true;
            break;
        }
    }
    assert!(found_note, "preview must show the binary meta note");

    let header = row_text(&buffer, 0);
    assert!(
        !header.contains('┬'),
        "binary preview header must not have a middle `┬` connector: {header:?}"
    );
}

#[test]
fn preview_submodule_single_column() {
    let src = "diff --git a/vendor b/vendor\nindex bef0a93..3810002 160000\n--- a/vendor\n+++ b/vendor\n@@ -1 +1 @@\n-Subproject commit bef0a93b4abf0000000000000000000000000000\n+Subproject commit 381000237a5f0000000000000000000000000000\n";
    let buffer = render_preview_for_diff(src);

    let mut full = String::new();
    for y in 0..buffer.area.height {
        full.push_str(&row_text(&buffer, y));
        full.push('\n');
    }
    assert!(
        !full.contains("submodule change"),
        "preview must NOT show the removed `submodule change` note: {full:?}"
    );
    assert!(
        full.contains("subproject"),
        "preview top-frame must carry the `subproject` label: {full:?}"
    );

    let header = row_text(&buffer, 0);
    assert!(
        header.contains('┬'),
        "submodule preview header must now carry the middle `┬` (Modified-style chrome): {header:?}"
    );
}

#[test]
fn preview_mode_only_before_shows_content() {
    use crate::app::{build_preview_pair, DiffSpec};
    use crate::diff::parse_unified_diff;

    let src = "diff --git a/Cargo.toml b/Cargo.toml\nold mode 100644\nnew mode 100755\n";
    let files = parse_unified_diff(src);
    assert_eq!(files.len(), 1);
    let (ss, theme) = test_assets();
    let pair = build_preview_pair(&files[0], &DiffSpec::default(), &ss, &theme);

    assert!(
        !pair.before_is_note,
        "before side must not be flagged as a note for ModeOnly"
    );
    assert!(
        pair.after_is_note,
        "after side carries the mode-change note"
    );

    let cargo_lines = std::fs::read_to_string("Cargo.toml")
        .unwrap()
        .lines()
        .count();
    assert_eq!(
        pair.before_lines.len(),
        cargo_lines,
        "before column must mirror Cargo.toml content ({cargo_lines} lines), got {}",
        pair.before_lines.len()
    );
    assert_eq!(
        pair.after_lines.len(),
        1,
        "after column is the single-line mode-change note"
    );
    let after_text: String = pair.after_lines[0]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert!(
        after_text.contains("mode") || after_text.contains("executable"),
        "after note must describe the mode change: {after_text:?}"
    );
}
