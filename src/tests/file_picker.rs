use crate::app::{App, AppConfig};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn new_file_picker_app() -> App {
    App::new_with_source(
        Vec::new(),
        Vec::new(),
        AppConfig {
            filename: "picker".to_string(),
            source: String::new(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    )
}

#[test]
fn file_picker_lists_dirs_then_markdown_files_only() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-picker-test-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("notes")).unwrap();
    fs::write(root.join("README.md"), "# Demo\n").unwrap();
    fs::write(root.join("draft.markdown"), "# Draft\n").unwrap();
    fs::write(root.join("ignore.txt"), "nope\n").unwrap();

    let mut app = new_file_picker_app();

    assert!(app.open_file_picker(root.clone()));

    let labels: Vec<_> = app
        .file_picker_entries()
        .iter()
        .map(|entry| entry.label())
        .collect();
    assert!(labels.contains(&"notes/"));
    assert!(labels.contains(&"README.md"));
    assert!(labels.contains(&"draft.markdown"));
    assert!(!labels.contains(&"ignore.txt"));

    let notes_idx = labels.iter().position(|label| *label == "notes/").unwrap();
    let readme_idx = labels
        .iter()
        .position(|label| *label == "README.md")
        .unwrap();
    assert!(notes_idx < readme_idx);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_lists_markdown_files_from_subdirectories() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-test-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs/nested")).unwrap();
    fs::write(root.join("README.md"), "# Demo\n").unwrap();
    fs::write(root.join("docs/guide.md"), "# Guide\n").unwrap();
    fs::write(root.join("docs/nested/deep.markdown"), "# Deep\n").unwrap();
    fs::write(root.join("ignore.txt"), "nope\n").unwrap();

    let mut app = new_file_picker_app();

    assert!(app.open_fuzzy_file_picker(root.clone()));
    assert!(app.is_fuzzy_file_picker());

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert!(labels.contains(&"README.md"));
    assert!(labels.contains(&"docs/guide.md"));
    assert!(labels.contains(&"docs/nested/deep.markdown"));
    assert!(!labels.contains(&"ignore.txt"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn queued_fuzzy_picker_transitions_from_pending_to_loading_to_open() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-queued-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("README.md"), "# Demo\n").unwrap();
    fs::write(root.join("docs/guide.md"), "# Guide\n").unwrap();

    let mut app = new_file_picker_app();

    app.queue_fuzzy_file_picker(root.clone());
    assert!(app.has_pending_picker());
    assert_eq!(
        app.pending_picker_mode(),
        Some(crate::app::FilePickerMode::Fuzzy)
    );
    assert_eq!(app.pending_picker_dir(), Some(root.as_path()));
    assert!(!app.is_picker_loading());
    assert!(app.start_pending_picker_loading());
    assert!(app.is_picker_loading());
    app.age_picker_loading_by(std::time::Duration::from_secs(1));
    let mut opened = false;
    for _ in 0..50 {
        if app.poll_picker_loading() {
            opened = app.is_file_picker_open();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(opened);
    assert!(app.is_file_picker_open());
    assert!(app.is_fuzzy_file_picker());
    assert!(!app.has_pending_picker());
    assert!(!app.is_picker_loading());

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert!(labels.contains(&"README.md"));
    assert!(labels.contains(&"docs/guide.md"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_uses_depth_first_order_with_hidden_first() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-picker-order-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".private")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join(".draft.md"), "# Hidden\n").unwrap();
    fs::write(root.join(".private/alpha.md"), "# Private\n").unwrap();
    fs::write(root.join("README.md"), "# Demo\n").unwrap();
    fs::write(root.join("docs/guide.md"), "# Guide\n").unwrap();

    let mut app = new_file_picker_app();

    assert!(app.open_fuzzy_file_picker(root.clone()));

    let labels: Vec<_> = app
        .file_picker_filtered_indices()
        .iter()
        .map(|idx| app.file_picker_entries()[*idx].label())
        .collect();
    assert_eq!(
        labels,
        vec![
            ".draft.md",
            "README.md",
            ".private/alpha.md",
            "docs/guide.md",
        ]
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_picker_page_moves_jump_and_clamp_at_edges() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-picker-page-test-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for idx in 0..18 {
        fs::write(root.join(format!("file{idx:03}.md")), "# Demo\n").unwrap();
    }

    let mut app = new_file_picker_app();

    assert!(app.open_file_picker(root.clone()));
    let total = app.file_picker_filtered_indices().len();
    // 18 markdown files plus the leading ".." parent entry
    assert_eq!(total, 19);

    for expected in [5, 10, 15, 18, 18] {
        app.move_file_picker_page_down();
        assert_eq!(app.file_picker_index(), expected);
    }

    for expected in [13, 8, 3, 0, 0] {
        app.move_file_picker_page_up();
        assert_eq!(app.file_picker_index(), expected);
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fuzzy_file_picker_page_moves_jump_and_clamp_at_edges() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-fuzzy-page-test-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for idx in 0..20 {
        fs::write(root.join(format!("file{idx:03}.md")), "# Demo\n").unwrap();
    }

    let mut app = new_file_picker_app();

    assert!(app.open_fuzzy_file_picker(root.clone()));
    let total = app.file_picker_filtered_indices().len();
    assert_eq!(total, 20);

    for expected in [5, 10, 15, 19, 19] {
        app.move_file_picker_page_down();
        assert_eq!(app.file_picker_index(), expected);
    }

    for expected in [14, 9, 4, 0, 0] {
        app.move_file_picker_page_up();
        assert_eq!(app.file_picker_index(), expected);
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_picker_lists_extensionless_devops_files_from_extras() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("leaf-picker-devops-{unique}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for name in [
        "Dockerfile",
        "Dockerfile.prod",
        "Makefile",
        "docker-compose.yml",
        "notes",
    ] {
        fs::write(root.join(name), "x\n").unwrap();
    }

    let mut app = new_file_picker_app();
    app.set_extras(vec![
        "Dockerfile".to_string(),
        "makefile".to_string(),
        "yml".to_string(),
    ]);
    assert!(app.open_file_picker(root.clone()));

    let labels: Vec<_> = app
        .file_picker_entries()
        .iter()
        .map(|entry| entry.label())
        .collect();
    for expected in [
        "Dockerfile",
        "Dockerfile.prod",
        "Makefile",
        "docker-compose.yml",
    ] {
        assert!(labels.contains(&expected), "{expected} missing: {labels:?}");
    }
    assert!(!labels.contains(&"notes"), "{labels:?}");

    let _ = fs::remove_dir_all(root);
}
