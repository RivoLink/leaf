use super::test_assets;
use crate::app::{App, DiffFlash, DiffSource, DiffSpec};
use crate::diff::{parse_unified_diff, DiffLayout};
use crate::tests::common::git_scratch::ScratchRepo;
use ratatui::text::Line;
use syntect::highlighting::ThemeSet;

fn diff_app_with_spec(src: &str, spec: DiffSpec) -> App {
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
    app.content_area = ratatui::layout::Rect::new(0, 0, 80, 10);
    app.install_diff(
        parse_unified_diff(src),
        spec,
        std::sync::Arc::new(ss),
        std::sync::Arc::new(theme),
    );
    app
}

fn diff_app(src: &str) -> App {
    diff_app_with_spec(src, DiffSpec::default())
}

fn patch_spec() -> DiffSpec {
    DiffSpec {
        source: DiffSource::Patch("change.patch".into()),
        ..DiffSpec::default()
    }
}

fn many_files(needle_file: usize) -> String {
    let mut src = String::new();
    for i in 0..30 {
        let added = if i == needle_file { "needle" } else { "plain" };
        src.push_str(&format!(
            "diff --git a/f{i}.txt b/f{i}.txt\n--- a/f{i}.txt\n+++ b/f{i}.txt\n\
             @@ -1,3 +1,3 @@\n ctx{i}\n-old{i}\n+{added}{i}\n tail{i}\n"
        ));
    }
    src
}

fn plain(line: &Line<'_>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

#[test]
fn search_in_split_lands_on_the_match_row() {
    let (ss, theme) = test_assets();
    let mut app = diff_app(&many_files(20));
    app.build_split_now(&ss, &theme);
    assert!(app.diff_toggle_layout(&ss, &theme));

    app.set_search_query("needle");
    app.run_search();

    let state = app.diff().unwrap();
    assert_eq!(state.layout, DiffLayout::Split);
    let row = app.search_matches()[0];
    assert!(plain(&state.split_lines[row]).contains("needle20"));
    assert_eq!(app.diff_current_file_from_scroll(), Some(20));
}

#[test]
fn search_ignores_box_decorations() {
    let mut app = diff_app(&many_files(3));
    app.set_search_query("│");
    app.run_search();
    assert!(app.search_matches().is_empty());

    app.set_search_query("f3.txt");
    app.run_search();
    assert!(
        !app.search_matches().is_empty(),
        "file paths are searchable"
    );
}

#[test]
fn search_switches_index_with_layout() {
    let (ss, theme) = test_assets();
    let mut app = diff_app(&many_files(20));
    app.set_search_query("needle");
    app.run_search();
    app.build_split_now(&ss, &theme);
    assert!(app.diff_toggle_layout(&ss, &theme));

    let row = app.active_highlight_line().unwrap();
    assert!(plain(&app.diff().unwrap().split_lines[row]).contains("needle20"));
}

#[test]
fn search_is_disabled_in_preview() {
    let (ss, theme) = test_assets();
    let mut app = diff_app_with_spec(&many_files(20), patch_spec());
    assert!(app.diff_toggle_preview(&ss, &theme));

    app.set_search_query("needle");
    app.run_search();

    assert!(app.search_matches().is_empty());
    assert!(matches!(
        app.diff_flash().map(|(f, _)| f),
        Some(DiffFlash::SearchInPreview)
    ));
}

#[test]
fn patch_preview_never_reads_local_files() {
    let Some(repo) = ScratchRepo::new("leaf-patch-preview") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };
    let spec = DiffSpec {
        cwd: Some(repo.path().to_path_buf()),
        ..patch_spec()
    };
    repo.write_file("x.md", "keep\nold\n").unwrap();
    let Some(out) = repo.run(&["hash-object", "-w", "x.md"]) else {
        eprintln!("git hash-object failed; skipping");
        return;
    };
    let old_blob = String::from_utf8(out.stdout).unwrap();
    repo.write_file("x.md", "LOCAL CONTENT\n").unwrap();
    let src = format!(
        "diff --git a/x.md b/x.md\nindex {}..89abcde 100644\n--- a/x.md\n+++ b/x.md\n\
         @@ -1,2 +1,2 @@\n keep\n-old\n+new\n",
        old_blob.trim()
    );
    let files = parse_unified_diff(&src);
    let (ss, theme) = test_assets();

    let pair = crate::app::build_preview_pair(&files[0], &spec, &ss, &theme);

    let after: String = pair.after_lines.iter().map(plain).collect();
    assert!(!after.contains("LOCAL"), "{after:?}");
    assert!(after.contains("new"), "{after:?}");
}

#[test]
fn reconstruction_rejects_a_mismatching_source() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n keep\n-old\n+new\n";
    let files = parse_unified_diff(src);
    assert!(crate::app::reconstruct_after_from_hunks("keep\nold\n", &files[0]).is_ok());
    assert!(crate::app::reconstruct_after_from_hunks("other\nold\n", &files[0]).is_err());
}

#[test]
fn working_preview_from_a_subdirectory_reads_the_current_worktree_file() {
    let Some(repo) = ScratchRepo::new("leaf-preview-subdir") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };
    let spec = DiffSpec {
        cwd: Some(repo.path().join("sub")),
        ..Default::default()
    };
    std::fs::create_dir_all(repo.path().join("sub")).unwrap();
    repo.write_file("sub/f.txt", "one\n").unwrap();
    if repo.commit("sub/f.txt", "init").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }
    repo.write_file("sub/f.txt", "two\nthree\n").unwrap();
    let diff = crate::run_git_diff_subprocess(&spec).unwrap();
    // Not derivable from the hunks.
    repo.write_file("sub/f.txt", "two\nthree\nfour\n").unwrap();
    let files = parse_unified_diff(&diff);
    let (ss, theme) = test_assets();

    let pair = crate::app::build_preview_pair(&files[0], &spec, &ss, &theme);

    let after: String = pair.after_lines.iter().map(plain).collect();
    assert!(after.contains("four"), "{after:?}");
    assert!(pair.after_error.is_none());
}

#[test]
fn preview_opens_near_the_first_change() {
    let Some(repo) = ScratchRepo::new("leaf-preview-first-change") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };
    let spec = DiffSpec {
        cwd: Some(repo.path().to_path_buf()),
        ..Default::default()
    };
    let original: String = (1..=40).map(|i| format!("line {i}\n")).collect();
    repo.write_file("long.txt", &original).unwrap();
    if repo.commit("long.txt", "init").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }
    repo.write_file("long.txt", &original.replace("line 20\n", "changed 20\n"))
        .unwrap();
    let src = crate::run_git_diff_subprocess(&spec).unwrap();
    let mut app = diff_app_with_spec(&src, spec);
    let (ss, theme) = test_assets();

    assert!(app.diff_toggle_preview(&ss, &theme));

    let pair = &app.diff().unwrap().preview_cache[&0];
    assert_eq!(pair.first_change_row, 19);
    assert_eq!(app.scroll(), 16, "three rows of context above the change");
}

#[test]
fn deleted_comment_does_not_color_added_code() {
    let (ss, theme) = test_assets();
    let with_comment =
        "diff --git a/x.rs b/x.rs\n--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-/* open\n+let x = 1;\n";
    let without =
        "diff --git a/x.rs b/x.rs\n--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-foo\n+let x = 1;\n";
    let row = |src: &str| -> Line<'static> {
        let built = crate::render::build_diff_lines(&parse_unified_diff(src), &ss, &theme);
        built
            .lines
            .into_iter()
            .find(|l| plain(l).contains("let x"))
            .unwrap()
    };
    let a = row(with_comment);
    let b = row(without);
    let let_style = |line: &Line<'_>| {
        line.spans
            .iter()
            .find(|s| s.content.contains("let"))
            .map(|s| s.style.fg)
    };
    assert_eq!(let_style(&a), let_style(&b));
}

#[test]
fn syntax_is_detected_from_file_name_and_shebang() {
    let (ss, _) = test_assets();
    let makefile = parse_unified_diff(
        "diff --git a/Makefile b/Makefile\n--- a/Makefile\n+++ b/Makefile\n@@ -1 +1 @@\n-a:\n+b:\n",
    );
    assert_eq!(
        crate::render::resolve_file_syntax(&makefile[0], &ss).map(|s| s.name.as_str()),
        Some("Makefile")
    );
    let script = parse_unified_diff(
        "diff --git a/run b/run\n--- a/run\n+++ b/run\n@@ -1,2 +1,2 @@\n #!/usr/bin/env python3\n-a = 1\n+a = 2\n",
    );
    assert_eq!(
        crate::render::resolve_file_syntax(&script[0], &ss).map(|s| s.name.as_str()),
        Some("Python")
    );
}

#[test]
fn binary_preview_is_notes_only() {
    let (ss, theme) = test_assets();
    let files = parse_unified_diff(
        "diff --git a/i.png b/i.png\nindex 1111111..2222222 100644\nBinary files a/i.png and b/i.png differ\n",
    );
    let pair = crate::app::build_preview_pair(&files[0], &DiffSpec::default(), &ss, &theme);
    assert!(pair.before_is_note && pair.after_is_note);
}

#[test]
fn refresh_reruns_git_diff_in_cwd_with_paths() {
    let Some(repo) = ScratchRepo::new("leaf-diff-refresh") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };
    let spec = DiffSpec {
        cwd: Some(repo.path().join("sub")),
        paths: vec!["a.md".into()],
        ..Default::default()
    };
    std::fs::create_dir_all(repo.path().join("sub")).unwrap();
    repo.write_file("sub/a.md", "one\n").unwrap();
    repo.write_file("sub/b.md", "one\n").unwrap();
    if repo.commit("sub", "init").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }
    repo.write_file("sub/a.md", "two\n").unwrap();
    repo.write_file("sub/b.md", "excluded\n").unwrap();
    let src = crate::run_git_diff_subprocess(&spec).unwrap();
    let mut app = diff_app_with_spec(&src, spec);
    repo.write_file("sub/a.md", "three\n").unwrap();
    let (ss, _) = test_assets();
    let ts = ThemeSet::load_defaults();

    assert!(app.refresh_diff(&ss, &ts));

    let text: String = app
        .visible_lines(0, app.total())
        .iter()
        .map(plain)
        .collect();
    assert!(text.contains("three"), "{text:?}");
    assert!(!text.contains("excluded"), "{text:?}");

    repo.write_file("sub/a.md", "four\n").unwrap();
    let theme = &ts.themes["base16-ocean.dark"];
    assert!(app.diff_toggle_preview(&ss, theme));
    let pair = &app.diff().unwrap().preview_cache[&0];
    let after: String = pair.after_lines.iter().map(plain).collect();
    assert!(after.contains("four"), "{after:?}");
}

#[test]
fn search_match_is_not_hidden_by_the_sticky_header() {
    use ratatui::{backend::TestBackend, Terminal};
    let mut src = String::from(
        "diff --git a/big.txt b/big.txt\n--- a/big.txt\n+++ b/big.txt\n@@ -1,30 +1,30 @@\n",
    );
    for i in 0..30 {
        if i == 20 {
            src.push_str("-old\n+needle here\n");
        } else {
            src.push_str(&format!(" line {i}\n"));
        }
    }
    let mut app = diff_app(&src);
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    app.set_search_query("needle");
    app.run_search();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let screen: String = (0..12)
        .map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect::<String>() + "\n")
        .collect();
    assert!(screen.contains("needle here"), "{screen}");
}

#[test]
fn search_jump_selects_the_file_of_the_match() {
    let mut app = diff_app(&many_files(29));
    app.content_area = ratatui::layout::Rect::new(0, 0, 80, 30);
    app.set_search_query("needle");
    app.run_search();
    assert_eq!(app.diff_current_file_from_scroll(), Some(29));
}

#[test]
fn search_in_preview_keeps_file_stepping() {
    let (ss, theme) = test_assets();
    let mut app = diff_app_with_spec(&many_files(20), patch_spec());
    assert!(app.diff_toggle_preview(&ss, &theme));
    app.set_search_query("needle");
    app.run_search();
    assert!(!app.has_active_search(), "n/N must step files again");
}

#[test]
fn zero_context_insertion_goes_after_its_line() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -3,0 +4 @@\n+d\n";
    let files = parse_unified_diff(src);
    let after = crate::app::reconstruct_after_from_hunks("a\nb\nc\n", &files[0]).unwrap();
    assert_eq!(after, "a\nb\nc\nd\n");
}

#[test]
fn refresh_keeps_the_current_file_and_layout() {
    let Some(repo) = ScratchRepo::new("leaf-refresh-position") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };
    let spec = DiffSpec {
        cwd: Some(repo.path().to_path_buf()),
        ..Default::default()
    };
    for name in ["a.md", "b.md", "c.md"] {
        repo.write_file(name, "one\ntwo\n").unwrap();
    }
    if repo.commit(".", "init").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }
    for name in ["a.md", "b.md", "c.md"] {
        repo.write_file(name, "one\nTWO\n").unwrap();
    }
    let src = crate::run_git_diff_subprocess(&spec).unwrap();
    let mut app = diff_app_with_spec(&src, spec);
    let (ss, theme) = test_assets();
    app.build_split_now(&ss, &theme);
    assert!(app.diff_toggle_layout(&ss, &theme));
    assert!(app.diff_nav_to_file(2));
    let scroll_before = app.scroll();
    repo.write_file("a.md", "one\nTHREE\n").unwrap();
    let ts = ThemeSet::load_defaults();

    assert!(app.refresh_diff(&ss, &ts));

    let state = app.diff().unwrap();
    assert_eq!(state.layout, DiffLayout::Split);
    assert!(state.split_ready && !state.split_lines.is_empty());
    assert_eq!(app.diff_current_file_from_scroll(), Some(2));
    assert_eq!(app.scroll(), scroll_before, "same rows, same position");
}

fn patch_app(path: &std::path::Path, src: &str) -> App {
    let (ss, theme) = test_assets();
    let mut app = App::new_with_source(
        Vec::<Line<'static>>::new(),
        Vec::new(),
        crate::app::AppConfig {
            filename: "change.patch".to_string(),
            source: src.to_string(),
            debug_input: false,
            watch: false,
            filepath: Some(path.to_path_buf()),
            last_file_state: None,
        },
    );
    app.content_area = ratatui::layout::Rect::new(0, 0, 80, 10);
    app.install_diff(
        parse_unified_diff(src),
        patch_spec(),
        std::sync::Arc::new(ss),
        std::sync::Arc::new(theme),
    );
    app
}

fn numbered_files(count: usize) -> String {
    (0..count)
        .map(|i| {
            format!(
                "diff --git a/f{i}.txt b/f{i}.txt\n--- a/f{i}.txt\n+++ b/f{i}.txt\n\
                 @@ -1,3 +1,3 @@\n ctx{i}\n-old{i}\n+new{i} needle\n tail{i}\n"
            )
        })
        .collect()
}

#[test]
fn layout_switch_keeps_the_active_match() {
    let (ss, theme) = test_assets();
    let mut app = diff_app(&numbered_files(5));
    app.set_search_query("needle");
    app.run_search();
    app.next_match();
    app.next_match();
    assert_eq!(app.diff_current_file_from_scroll(), Some(2));
    app.build_split_now(&ss, &theme);
    assert!(app.diff_toggle_layout(&ss, &theme));
    assert_eq!(
        app.diff_current_file_from_scroll(),
        Some(2),
        "same match after `v`"
    );
    app.next_match();
    assert_eq!(app.diff_current_file_from_scroll(), Some(3));
}

#[test]
fn reload_stays_on_a_deleted_file() {
    let (ss, _) = test_assets();
    let ts = ThemeSet::load_defaults();
    let src: String = (1..=3)
        .map(|i| {
            format!(
                "diff --git a/d{i} b/d{i}\ndeleted file mode 100644\n--- a/d{i}\n+++ /dev/null\n\
                 @@ -1 +0,0 @@\n-gone{i}\n"
            )
        })
        .collect();
    let dir = super::unique_temp_dir("leaf-reload-deleted");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("change.patch");
    std::fs::write(&path, &src).unwrap();
    let mut app = patch_app(&path, &src);
    assert!(app.diff_nav_to_file(2));

    assert!(app.reload(&ss, &ts));

    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(app.diff_current_file_from_scroll(), Some(2));
}

#[test]
fn reload_after_moving_in_the_preview_lands_on_that_file() {
    let (ss, theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let src = numbered_files(8);
    let dir = super::unique_temp_dir("leaf-reload-preview");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("change.patch");
    std::fs::write(&path, &src).unwrap();
    let mut app = patch_app(&path, &src);
    assert!(app.diff_nav_to_file(6));
    assert!(app.diff_toggle_preview(&ss, &theme));
    for _ in 0..3 {
        assert!(app.diff_preview_step_file(-1, &ss, &theme));
    }

    assert!(app.reload(&ss, &ts));
    assert!(app.diff_toggle_preview(&ss, &theme));

    let _ = std::fs::remove_dir_all(&dir);
    let state = app.diff().unwrap();
    assert_eq!(state.current_file_idx, 3);
    let top = state.first_scroll_target_of_file(3).unwrap();
    assert_eq!(app.scroll(), top.min(app.max_scroll()));
}

#[test]
fn reload_when_the_current_file_disappears_goes_to_its_neighbour() {
    let (ss, _) = test_assets();
    let ts = ThemeSet::load_defaults();
    let dir = super::unique_temp_dir("leaf-reload-gone");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("change.patch");
    std::fs::write(&path, numbered_files(3)).unwrap();
    let mut app = patch_app(&path, &numbered_files(3));
    assert!(app.diff_nav_to_file(2));
    std::fs::write(&path, numbered_files(2)).unwrap();

    assert!(app.reload(&ss, &ts));

    let _ = std::fs::remove_dir_all(&dir);
    let state = app.diff().unwrap();
    assert_eq!(state.current_file_idx, 1);
    let top = state.first_scroll_target_of_file(1).unwrap();
    assert_eq!(app.scroll(), top.min(app.max_scroll()));
}

fn active_match_text(app: &App) -> String {
    let state = app.diff().unwrap();
    let row = app.active_highlight_line().unwrap();
    match state.layout {
        DiffLayout::Unified => state.search_lines[row].clone(),
        DiffLayout::Split => state.split_search_lines[row].clone(),
    }
}

fn toggle(app: &mut App) {
    let (ss, theme) = test_assets();
    app.build_split_now(&ss, &theme);
    assert!(app.diff_toggle_layout(&ss, &theme));
}

#[test]
fn layout_round_trip_keeps_the_added_line_match() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n-dd1\n-dd2\n+aa1\n+aa2\n";
    let mut app = diff_app(src);
    app.set_search_query("aa");
    app.run_search();
    app.next_match();
    assert!(active_match_text(&app).contains("aa2"));
    toggle(&mut app);
    assert!(active_match_text(&app).contains("aa2"));
    toggle(&mut app);
    let text = active_match_text(&app);
    assert!(text.contains("aa2") && !text.contains("aa1"), "{text}");
}

#[test]
fn layout_round_trip_is_stable_when_both_sides_match() {
    let src = "diff --git a/y b/y\n--- a/y\n+++ b/y\n@@ -1 +1 @@\n-dx2\n+ax2\n";
    let mut app = diff_app(src);
    app.set_search_query("x2");
    app.run_search();
    app.next_match();
    assert!(active_match_text(&app).contains("ax2"));
    toggle(&mut app);
    toggle(&mut app);
    let text = active_match_text(&app);
    assert!(text.contains("ax2") && !text.contains("dx2"), "{text}");
}

#[test]
fn layout_switch_keeps_a_hunk_header_match() {
    let body = |n: usize| -> String {
        (0..3)
            .map(|i| format!(" c{n}{i}\n"))
            .chain([format!("-o{n}\n+n{n}\n")])
            .collect()
    };
    let src = format!(
        "diff --git a/h b/h\n--- a/h\n+++ b/h\n@@ -1,4 +1,4 @@\n{}@@ -40,4 +40,4 @@\n{}",
        body(1),
        body(2)
    );
    let mut app = diff_app(&src);
    app.set_search_query("around line");
    app.run_search();
    assert_eq!(app.search_match_count(), 2);
    app.next_match();
    toggle(&mut app);
    assert_eq!(app.search_match_count(), 2);
    assert_eq!(app.search_index(), 1, "the second hunk header stays active");
    toggle(&mut app);
    assert_eq!(app.search_index(), 1);
}

#[test]
fn reload_follows_a_modified_file_that_became_deleted() {
    let (ss, _) = test_assets();
    let ts = ThemeSet::load_defaults();
    let modified = |name: &str| {
        format!("diff --git a/{name} b/{name}\n--- a/{name}\n+++ b/{name}\n@@ -1 +1 @@\n-o\n+n\n")
    };
    let before = format!("{}{}{}", modified("a"), modified("f"), modified("z"));
    let after = format!(
        "{}diff --git a/b b/b\nnew file mode 100644\n--- /dev/null\n+++ b/b\n@@ -0,0 +1 @@\n+b\n\
         diff --git a/f b/f\ndeleted file mode 100644\n--- a/f\n+++ /dev/null\n@@ -1 +0,0 @@\n-o\n{}",
        modified("a"),
        modified("z")
    );
    let dir = super::unique_temp_dir("leaf-reload-became-deleted");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("change.patch");
    std::fs::write(&path, &before).unwrap();
    let mut app = patch_app(&path, &before);
    assert!(app.diff_nav_to_file(1));
    std::fs::write(&path, &after).unwrap();

    assert!(app.reload(&ss, &ts));

    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(app.diff_current_file_from_scroll(), Some(2));
}

#[test]
fn match_missing_from_the_other_layout_converts_to_the_nearest_one() {
    let file = |name: &str| {
        format!("diff --git a/{name} b/{name}\n--- a/{name}\n+++ b/{name}\n@@ -1 +1 @@\n-o\n+zqneedle\n")
    };
    let mut app = diff_app(&format!("{}{}{}", file("f0"), file("f1"), file("f2")));
    app.set_search_query("zqneedle");
    app.run_search();
    app.prev_match();
    let row = app.active_highlight_line().unwrap();
    let (ss, theme) = test_assets();
    app.build_split_now(&ss, &theme);
    let state = app.diff().unwrap();
    let split_matches: Vec<usize> = state
        .split_search_lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("zqneedle"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(split_matches.len(), 3);
    let without_last = &split_matches[..2];
    assert_eq!(
        state.convert_match_row(DiffLayout::Unified, DiffLayout::Split, row, without_last),
        Some(split_matches[1])
    );
    assert_eq!(
        state.convert_match_row(DiffLayout::Unified, DiffLayout::Split, row, &[]),
        None
    );
}

fn activate(app: &mut App, needle: &str) {
    for _ in 0..app.search_match_count() {
        if active_match_text(app).contains(needle) {
            return;
        }
        app.next_match();
    }
    panic!("no active match containing {needle:?}");
}

#[test]
fn layout_switch_keeps_a_trailing_no_newline_note() {
    let tail = "-old\n\\ No newline at end of file\n+new\n\\ No newline at end of file\n";
    let context: String = (0..10).map(|i| format!(" c{i}\n")).collect();
    let src = format!(
        "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n{tail}\
         diff --git a/b b/b\n--- a/b\n+++ b/b\n@@ -1,11 +1,11 @@\n{context}{tail}"
    );
    let mut app = diff_app(&src);
    app.set_search_query("newline");
    app.run_search();
    app.prev_match();
    assert_eq!(app.diff_current_file_from_scroll(), Some(1));
    let last = app.search_index();
    toggle(&mut app);
    assert_eq!(app.diff_current_file_from_scroll(), Some(1));
    assert!(active_match_text(&app).contains("newline"));
    toggle(&mut app);
    assert_eq!(app.search_index(), last);
}

#[test]
fn layout_switch_keeps_a_merge_conflict_row() {
    let src = "diff --cc m.txt\nindex 3659f2c,1abd9d7..0000000\n--- a/m.txt\n+++ b/m.txt\n\
               @@@ -1,3 -1,3 +1,7 @@@\n  A\n++<<<<<<< HEAD\n +y1\n++=======\n+ x1\n++>>>>>>> x\n  B\n";
    let mut app = diff_app(src);
    app.set_search_query("1");
    app.run_search();
    activate(&mut app, "y1");
    toggle(&mut app);
    assert!(
        active_match_text(&app).contains("y1"),
        "{}",
        active_match_text(&app)
    );
    toggle(&mut app);
    assert!(
        active_match_text(&app).contains("y1"),
        "{}",
        active_match_text(&app)
    );
}

#[test]
fn new_query_forgets_the_last_layout_switch() {
    let src = "diff --git a/y b/y\n--- a/y\n+++ b/y\n@@ -1 +1 @@\n-dx2\n+ax2\n";
    let mut app = diff_app(src);
    app.set_search_query("x2");
    app.run_search();
    app.next_match();
    toggle(&mut app);
    app.set_search_query("2");
    app.run_search();
    activate(&mut app, "dx2");
    toggle(&mut app);
    let text = active_match_text(&app);
    assert!(text.contains("dx2") && !text.contains("ax2"), "{text}");
}

#[test]
fn layout_switch_in_a_merge_file_keeps_the_same_row() {
    let conflict = |n: usize| format!("++<<<<<<< HEAD\n +qa{n}\n++=======\n+ qb{n}\n++>>>>>>> x\n");
    let context: String = (1..=10).map(|i| format!("  ctx{i}\n")).collect();
    let src = format!(
        "diff --cc m.txt\nindex 3659f2c,1abd9d7..0000000\n--- a/m.txt\n+++ b/m.txt\n\
         @@@ -1,13 -1,13 +1,23 @@@\n  A\n{}{context}{}  Z\n",
        conflict(1),
        conflict(2)
    );
    for (query, needle) in [("q", "qa1"), ("q", "qa2"), ("q", "qb2"), ("ctx", "ctx3")] {
        for start_split in [false, true] {
            let mut app = diff_app(&src);
            if start_split {
                toggle(&mut app);
            }
            app.set_search_query(query);
            app.run_search();
            activate(&mut app, needle);
            toggle(&mut app);
            let text = active_match_text(&app);
            assert!(
                text.contains(needle),
                "{needle} from split={start_split}: {text}"
            );
        }
    }
}
