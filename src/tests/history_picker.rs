use crate::app::{history::HistoryEntry, App, AppConfig};

#[test]
fn history_picker_page_moves_jump_and_clamp_at_edges() {
    let mut app = App::new_with_source(
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
    );

    app.set_file_history_length(Some(25));
    let entries: Vec<HistoryEntry> = (0..25)
        .map(|idx| HistoryEntry {
            path: std::env::temp_dir().join(format!("leaf-hist-{idx:03}.md")),
        })
        .collect();
    app.install_loaded_history_picker(entries);
    assert_eq!(app.history_picker_filtered_indices().len(), 25);

    app.move_history_picker_page_down();
    assert_eq!(app.history_picker_index(), 10);
    app.move_history_picker_page_down();
    assert_eq!(app.history_picker_index(), 20);
    app.move_history_picker_page_down();
    assert_eq!(app.history_picker_index(), 24);

    app.move_history_picker_page_up();
    assert_eq!(app.history_picker_index(), 14);
    app.move_history_picker_page_up();
    assert_eq!(app.history_picker_index(), 4);
    app.move_history_picker_page_up();
    assert_eq!(app.history_picker_index(), 0);
    app.move_history_picker_page_up();
    assert_eq!(app.history_picker_index(), 0);
}
