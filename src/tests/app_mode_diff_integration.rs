use crate::app::{App, DiffSpec};
use crate::diff::parse_unified_diff;
use crate::tests::common::git_scratch::ScratchRepo;
use ratatui::text::Line;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

fn make_app() -> App {
    App::new(
        Vec::<Line<'static>>::new(),
        Vec::new(),
        "stdin".to_string(),
        false,
        false,
        None,
        None,
    )
}

fn syntect_defaults() -> (SyntaxSet, syntect::highlighting::Theme) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = ts.themes["base16-ocean.dark"].clone();
    (ss, theme)
}

fn flatten_lines(lines: &[Line<'static>]) -> String {
    lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn build_preview_pair_before_and_after_are_distinct_for_modified_file() {
    let Some(repo) = ScratchRepo::new("leaf-preview-test") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };
    repo.write_file("config.toml", "version = \"0.1.0\"\nverbose = false\n")
        .unwrap();
    if repo.commit("config.toml", "init").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }
    repo.write_file("config.toml", "version = \"0.2.0\"\nverbose = true\n")
        .unwrap();
    let Some(diff_out) = repo.run(&["diff", "config.toml"]) else {
        eprintln!("git diff failed; skipping");
        return;
    };
    let diff_src = String::from_utf8(diff_out.stdout).unwrap();
    let files = parse_unified_diff(&diff_src);
    assert_eq!(files.len(), 1, "one file diff expected");
    let (ss, theme) = syntect_defaults();
    let spec = DiffSpec {
        cwd: Some(repo.path().to_path_buf()),
        ..Default::default()
    };

    let pair = crate::app::build_preview_pair(&files[0], &spec, &ss, &theme);
    let before_str = flatten_lines(&pair.before_lines);
    let after_str = flatten_lines(&pair.after_lines);
    assert!(
        before_str.contains("0.1.0"),
        "BEFORE column should contain HEAD version 0.1.0: {before_str:?}"
    );
    assert!(
        after_str.contains("0.2.0"),
        "AFTER column should contain working version 0.2.0: {after_str:?}"
    );
    assert!(
        !pair.before_lines.is_empty() && !pair.after_lines.is_empty(),
        "both preview columns must be non-empty for a modified file"
    );
    assert_ne!(
        before_str, after_str,
        "BEFORE and AFTER columns must render distinct content"
    );

    use ratatui::{backend::TestBackend, Terminal};
    let mut app = make_app();
    app.set_source_for_test(diff_src.clone());
    app.install_diff(
        files.clone(),
        spec.clone(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    let flipped = app.diff_toggle_preview(&ss, &theme);
    assert!(flipped);
    let mut terminal = Terminal::new(TestBackend::new(120, 20)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let w = buffer.area.width as usize;
    let h = buffer.area.height as usize;
    let mid = w / 2;
    let mut left_text = String::new();
    let mut right_text = String::new();

    for y in 1..h {
        for x in 0..mid {
            left_text.push_str(buffer.cell((x as u16, y as u16)).unwrap().symbol());
        }
        left_text.push('\n');

        for x in mid..w {
            right_text.push_str(buffer.cell((x as u16, y as u16)).unwrap().symbol());
        }
        right_text.push('\n');
    }
    assert!(
        left_text.contains("0.1.0"),
        "left column should show HEAD version 0.1.0, got: {left_text:?}"
    );
    assert!(
        right_text.contains("0.2.0"),
        "right column should show working version 0.2.0, got: {right_text:?}"
    );
}

#[test]
fn build_preview_pair_markdown_before_and_after_are_distinct() {
    let Some(repo) = ScratchRepo::new("leaf-preview-md-test") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };
    std::fs::create_dir_all(repo.path().join("docs")).unwrap();
    repo.write_file(
        "docs/notes.md",
        "# Notes\n\nOriginal version alpha.\n\n## Status\n\nInitial.\n",
    )
    .unwrap();
    if repo.commit("docs/notes.md", "init").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }
    repo.write_file(
        "docs/notes.md",
        "# Notes\n\nModified version omega.\n\n## Status\n\nUpdated with extras.\n\n- one\n- two\n",
    )
    .unwrap();
    let Some(diff_out) = repo.run(&["diff", "docs/notes.md"]) else {
        eprintln!("git diff failed; skipping");
        return;
    };
    let diff_src = String::from_utf8(diff_out.stdout).unwrap();
    let files = parse_unified_diff(&diff_src);
    assert_eq!(files.len(), 1, "one file diff expected");
    let (ss, theme) = syntect_defaults();
    let spec = DiffSpec {
        cwd: Some(repo.path().to_path_buf()),
        ..Default::default()
    };

    let pair = crate::app::build_preview_pair(&files[0], &spec, &ss, &theme);
    let before_str = flatten_lines(&pair.before_lines);
    let after_str = flatten_lines(&pair.after_lines);
    assert!(
        before_str.contains("alpha"),
        "BEFORE column should contain HEAD marker 'alpha': {before_str:?}"
    );
    assert!(
        after_str.contains("omega"),
        "AFTER column should contain working marker 'omega': {after_str:?}"
    );
    assert!(
        !pair.before_lines.is_empty() && !pair.after_lines.is_empty(),
        "both preview columns must be non-empty for a modified markdown file"
    );

    use ratatui::{backend::TestBackend, Terminal};
    let mut app = make_app();
    app.set_source_for_test(diff_src.clone());
    app.install_diff(
        files.clone(),
        spec.clone(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );

    app.scroll_down(100);
    assert!(
        app.scroll() > 0,
        "pre-condition: diff should be scrolled before preview toggles on"
    );
    let flipped = app.diff_toggle_preview(&ss, &theme);
    assert!(flipped);
    assert_eq!(
        app.scroll(),
        0,
        "toggling the preview on must reset the shared scroll to 0"
    );
    let mut terminal = Terminal::new(TestBackend::new(140, 24)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let w = buffer.area.width as usize;
    let h = buffer.area.height as usize;
    let mid = w / 2;
    let mut left_text = String::new();
    let mut right_text = String::new();
    for y in 2..h {
        for x in 0..mid {
            left_text.push_str(buffer.cell((x as u16, y as u16)).unwrap().symbol());
        }
        left_text.push('\n');

        for x in mid..w {
            right_text.push_str(buffer.cell((x as u16, y as u16)).unwrap().symbol());
        }
        right_text.push('\n');
    }
    assert!(
        left_text.contains("alpha"),
        "LEFT column must render HEAD marker 'alpha', got:\n{left_text}"
    );
    assert!(
        right_text.contains("omega"),
        "RIGHT column must render working marker 'omega', got:\n{right_text}"
    );
    assert!(
        !left_text.contains("omega"),
        "LEFT column must not leak AFTER marker 'omega', got:\n{left_text}"
    );
}

#[test]
fn preview_panel_short_column_stays_pinned_on_deep_scroll() {
    let Some(repo) = ScratchRepo::new("leaf-preview-scroll-test") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };

    repo.write_file("short.md", "# Head\n\nBEFORE_MARK alpha.\n")
        .unwrap();
    if repo.commit("short.md", "init").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }

    let mut long = String::from("# Head\n\nAFTER_MARK omega.\n\n");
    for i in 0..40 {
        long.push_str(&format!("Line number {i} added.\n"));
    }
    repo.write_file("short.md", &long).unwrap();
    let Some(diff_out) = repo.run(&["diff", "short.md"]) else {
        eprintln!("git diff failed; skipping");
        return;
    };
    let diff_src = String::from_utf8(diff_out.stdout).unwrap();
    let files = parse_unified_diff(&diff_src);
    assert_eq!(files.len(), 1);
    let (ss, theme) = syntect_defaults();
    let spec = DiffSpec {
        cwd: Some(repo.path().to_path_buf()),
        ..Default::default()
    };

    use ratatui::{backend::TestBackend, Terminal};
    let mut app = make_app();
    app.set_source_for_test(diff_src.clone());
    app.install_diff(
        files.clone(),
        spec.clone(),
        std::sync::Arc::new(ss.clone()),
        std::sync::Arc::new(theme.clone()),
    );
    assert!(app.diff_toggle_preview(&ss, &theme));

    let mut terminal = Terminal::new(TestBackend::new(140, 24)).unwrap();
    terminal.draw(|f| crate::render::ui(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let w = buffer.area.width as usize;
    let h = buffer.area.height as usize;
    let mid = w / 2;
    let mut left_text = String::new();
    let mut right_text = String::new();
    for y in 2..h {
        for x in 0..mid {
            left_text.push_str(buffer.cell((x as u16, y as u16)).unwrap().symbol());
        }
        left_text.push('\n');
        for x in mid..w {
            right_text.push_str(buffer.cell((x as u16, y as u16)).unwrap().symbol());
        }
        right_text.push('\n');
    }
    assert!(
        left_text.contains("BEFORE_MARK"),
        "LEFT (old) column must render BEFORE marker, got:\n{left_text}"
    );
    assert!(
        right_text.contains("AFTER_MARK"),
        "RIGHT (new) column must render AFTER marker, got:\n{right_text}"
    );
}

#[test]
fn fetch_before_reads_the_old_blob_in_cwd() {
    let Some(repo) = ScratchRepo::new("leaf-fetch-before") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };

    // The hunk can't reach the first line, so only the blob can supply it.
    let v1 = "v1\na\nb\nc\nd\ne\nf\ng\nh\n";
    repo.write_file("f.txt", v1).unwrap();
    if repo.commit("f.txt", "v1").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }
    repo.write_file("f.txt", &v1.replace("h\n", "v2\n"))
        .unwrap();
    if repo.commit("f.txt", "v2").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }

    let diff_out = repo
        .run(&["diff", "HEAD~1", "HEAD", "--", "f.txt"])
        .expect("diff");
    let diff_text = String::from_utf8_lossy(&diff_out.stdout).into_owned();
    let files = parse_unified_diff(&diff_text);
    assert_eq!(files.len(), 1);
    assert!(files[0].old_blob_sha.is_some(), "index line parsed");

    let before = crate::app::fetch_before(
        &files[0],
        &DiffSpec {
            cwd: Some(repo.path().to_path_buf()),
            ..Default::default()
        },
    );
    assert_eq!(
        before, v1,
        "fetch_before must return the pre-image pinned by the blob SHA"
    );
}

#[test]
fn git_diff_subprocess_ignores_color_and_prefix_config() {
    let Some(repo) = ScratchRepo::new("leaf-git-diff-config") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };
    let spec = DiffSpec {
        cwd: Some(repo.path().to_path_buf()),
        ..Default::default()
    };
    repo.write_file("notes.md", "one\n").unwrap();
    if repo.commit("notes.md", "init").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }
    repo.write_file("notes.md", "two\n").unwrap();
    for (key, value) in [
        ("color.ui", "always"),
        ("color.diff", "always"),
        ("diff.mnemonicPrefix", "true"),
    ] {
        if repo.run(&["config", key, value]).is_none() {
            eprintln!("git config failed; skipping");
            return;
        }
    }

    let out = crate::run_git_diff_subprocess(&spec).unwrap();
    assert!(!out.contains('\x1b'), "no ANSI escapes expected: {out:?}");
    let files = parse_unified_diff(&out);
    assert_eq!(files.len(), 1, "{out:?}");
    assert_eq!(files[0].new_path, "notes.md");
}

#[test]
fn git_diff_subprocess_ignores_relative_config() {
    let Some(repo) = ScratchRepo::new("leaf-git-diff-relative") else {
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
    repo.write_file("sub/f.txt", "two\n").unwrap();
    if repo.run(&["config", "diff.relative", "true"]).is_none() {
        eprintln!("git config failed; skipping");
        return;
    }

    let out = crate::run_git_diff_subprocess(&spec).unwrap();
    let files = parse_unified_diff(&out);
    assert_eq!(
        files[0].new_path, "sub/f.txt",
        "paths stay relative to the repo root"
    );
}

#[test]
fn git_diff_subprocess_compares_two_revisions_with_paths() {
    let Some(repo) = ScratchRepo::new("leaf-git-diff-revs") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };
    let spec = DiffSpec {
        cwd: Some(repo.path().to_path_buf()),
        ..Default::default()
    };
    repo.write_file("a.md", "one\n").unwrap();
    repo.write_file("b.md", "one\n").unwrap();
    if repo.commit(".", "init").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }
    repo.write_file("a.md", "two\n").unwrap();
    repo.write_file("b.md", "two\n").unwrap();
    if repo.commit(".", "second").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }

    assert!(crate::is_git_revision(&spec, "HEAD~1"));
    assert!(!crate::is_git_revision(&spec, "a.md"));

    let spec = DiffSpec {
        source: crate::app::DiffSource::Revs("HEAD~1".into(), "HEAD".into()),
        paths: vec!["a.md".into()],
        ..spec
    };
    let out = crate::run_git_diff_subprocess(&spec).unwrap();
    let files = parse_unified_diff(&out);
    assert_eq!(files.len(), 1, "{out:?}");
    assert_eq!(files[0].new_path, "a.md");
}

#[test]
fn existing_path_wins_over_a_revision_of_the_same_name() {
    let Some(repo) = ScratchRepo::new("leaf-rev-or-path") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };
    let spec = DiffSpec {
        cwd: Some(repo.path().to_path_buf()),
        ..Default::default()
    };
    repo.write_file("d", "one\n").unwrap();
    if repo.commit("d", "init").is_none() || repo.run(&["branch", "d"]).is_none() {
        eprintln!("git setup failed; skipping");
        return;
    }
    assert!(crate::is_git_revision(&spec, "d"));

    let mut spec = DiffSpec {
        source: crate::app::DiffSource::Ref("HEAD".into()),
        cwd: Some(repo.path().to_path_buf()),
        ..DiffSpec::default()
    };
    crate::apply_diff_positional(&mut spec, true, "d".into());
    assert_eq!(spec.source, crate::app::DiffSource::Ref("HEAD".into()));
    assert_eq!(spec.paths, vec!["d".to_string()]);

    std::fs::remove_file(repo.path().join("d")).unwrap();
    let mut spec = DiffSpec {
        source: crate::app::DiffSource::Ref("HEAD".into()),
        cwd: Some(repo.path().to_path_buf()),
        ..DiffSpec::default()
    };
    crate::apply_diff_positional(&mut spec, true, "d".into());
    assert_eq!(
        spec.source,
        crate::app::DiffSource::Revs("HEAD".into(), "d".into())
    );
}

#[test]
fn scratch_repo_leaves_process_cwd_unchanged() {
    let cwd = std::env::current_dir().unwrap();
    let Some(repo) = ScratchRepo::new("leaf-cwd-test") else {
        eprintln!("git/scratch unavailable; skipping");
        return;
    };
    repo.write_file("f.txt", "v1\n").unwrap();
    if repo.commit("f.txt", "init").is_none() {
        eprintln!("git commit failed; skipping");
        return;
    }
    assert_eq!(std::env::current_dir().unwrap(), cwd);
}
