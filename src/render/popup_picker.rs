use crate::{app::App, editor::EditorKind, theme::app_theme};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use super::centered_rect;
use super::popup::{
    highlighted_picker_label, picker_truncation_message, popup_footer, popup_footer_line,
};

fn file_picker_area(f: &Frame, app: &mut App) -> Rect {
    let full = f.area();
    let width = app.refresh_picker_width_floor(full.width);
    centered_rect(width, 20, full)
}

pub(super) fn render_file_popup(f: &mut Frame, app: &mut App) {
    let theme = app_theme();
    let area = file_picker_area(f, app);
    let title_style = Style::default()
        .fg(theme.markdown.heading_2)
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default().fg(theme.ui.status_shortcut_fg);

    let inner_height = area.height.saturating_sub(2) as usize;
    let header_lines = if app.is_fuzzy_file_picker() { 4 } else { 3 };
    let total = app.file_picker_filtered_indices().len();
    let truncation_message = picker_truncation_message(app.file_picker_truncation());
    let max_visible_slots = if app.is_fuzzy_file_picker() {
        if truncation_message.is_some() {
            11
        } else {
            12
        }
    } else {
        13
    };
    let reserved_footer_lines = if truncation_message.is_some() { 3 } else { 2 };
    let visible_slots = inner_height
        .saturating_sub(header_lines + reserved_footer_lines)
        .min(max_visible_slots);
    let start = if visible_slots == 0 || app.file_picker_index() < visible_slots {
        0
    } else {
        app.file_picker_index() + 1 - visible_slots
    };
    let end = (start + visible_slots).min(total);

    let mut lines = vec![
        Line::from(vec![Span::styled("Open a Markdown file", title_style)]),
        Line::from(vec![
            Span::styled("Dir: ", section_style),
            Span::styled(
                app.file_picker_dir().display().to_string(),
                Style::default().fg(theme.ui.toc_primary_inactive),
            ),
        ]),
    ];

    if app.is_fuzzy_file_picker() {
        lines.push(Line::from(vec![
            Span::styled("Query: ", section_style),
            Span::styled(
                if app.file_picker_query().is_empty() {
                    " type to filter ".to_string()
                } else {
                    format!(" {} ", app.file_picker_query())
                },
                Style::default()
                    .fg(if app.file_picker_query().is_empty() {
                        theme.ui.toc_primary_inactive
                    } else {
                        theme.ui.toc_primary_active
                    })
                    .bg(theme.markdown.inline_code_bg),
            ),
        ]));
    }

    lines.push(Line::from(""));

    if app.file_picker_entries().is_empty() {
        lines.push(Line::from(vec![Span::styled(
            if app.is_fuzzy_file_picker() {
                "No Markdown file found in this directory or its subdirectories"
            } else {
                "No folders or Markdown files here"
            },
            Style::default().fg(theme.ui.toc_primary_inactive),
        )]));
    } else if total == 0 {
        lines.push(Line::from(vec![Span::styled(
            "No match for the current query",
            Style::default().fg(theme.ui.toc_primary_inactive),
        )]));
    } else {
        for (idx, entry_idx) in app.file_picker_filtered_indices()[start..end]
            .iter()
            .enumerate()
        {
            let actual_idx = start + idx;
            let selected = actual_idx == app.file_picker_index();
            let entry = &app.file_picker_entries()[*entry_idx];
            let bg = if selected {
                theme.ui.toc_active_bg
            } else {
                theme.ui.toc_bg
            };
            let marker = if selected { "▎ " } else { "  " };
            let label_spans = if app.is_fuzzy_file_picker() {
                highlighted_picker_label(
                    entry.label(),
                    app.file_picker_match_positions(actual_idx),
                    bg,
                    selected,
                )
            } else {
                vec![Span::styled(
                    entry.label().to_string(),
                    Style::default()
                        .fg(theme.ui.toc_primary_inactive)
                        .bg(bg)
                        .add_modifier(if selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                )]
            };
            let mut spans = vec![Span::styled(
                marker,
                Style::default()
                    .fg(theme.ui.toc_accent)
                    .bg(bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            )];
            spans.extend(label_spans);
            lines.push(Line::from(spans));
        }
    }

    while lines.len() < inner_height.saturating_sub(reserved_footer_lines) {
        lines.push(Line::from(""));
    }

    if let Some(message) = truncation_message {
        lines.push(Line::from(vec![Span::styled(
            "",
            Style::default().fg(theme.ui.toc_primary_inactive),
        )]));
        lines.push(Line::from(vec![Span::styled(
            message,
            Style::default().fg(theme.markdown.heading_3),
        )]));
    } else {
        lines.push(Line::from(""));
    }

    lines.push(popup_footer_line(
        popup_footer(app.has_content(), app.is_fuzzy_file_picker(), false),
        theme.ui.toc_bg,
    ));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title("─ Files ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}

pub(super) fn render_picker_loading_popup(f: &mut Frame, app: &mut App) {
    let theme = app_theme();
    let area = file_picker_area(f, app);
    let title_style = Style::default()
        .fg(theme.markdown.heading_2)
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default().fg(theme.ui.status_shortcut_fg);

    let is_failed = app.is_picker_load_failed();
    let is_fuzzy = matches!(
        app.pending_picker_mode(),
        Some(crate::app::FilePickerMode::Fuzzy)
    );
    let inner_height = area.height.saturating_sub(2) as usize;
    let message = if is_failed {
        app.picker_load_error().unwrap_or("Failed to load files")
    } else {
        "Indexing markdown files..."
    };

    let mut lines = vec![
        Line::from(vec![Span::styled("Open a Markdown file", title_style)]),
        Line::from(vec![
            Span::styled("Dir: ", section_style),
            Span::styled(
                app.pending_picker_dir()
                    .map(|dir| dir.display().to_string())
                    .unwrap_or_else(|| ".".to_string()),
                Style::default().fg(theme.ui.toc_primary_inactive),
            ),
        ]),
    ];

    if is_fuzzy {
        lines.push(Line::from(vec![
            Span::styled("Query: ", section_style),
            Span::styled(
                " type to filter ",
                Style::default()
                    .fg(theme.ui.toc_primary_inactive)
                    .bg(theme.markdown.inline_code_bg),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        message,
        Style::default().fg(theme.ui.toc_primary_inactive),
    )]));

    while lines.len() < inner_height.saturating_sub(2) {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(""));
    lines.push(popup_footer_line(
        popup_footer(app.has_content(), is_fuzzy, is_failed),
        theme.ui.toc_bg,
    ));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title("─ Files ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}

pub(super) fn render_editor_popup(f: &mut Frame, app: &App) {
    let theme = app_theme();
    let entries = app.editor_picker_entries();
    let selected = app.editor_picker_index();
    let current_editor = app.editor_config().map(crate::editor::binary_name);

    let section_style = Style::default()
        .fg(theme.ui.toc_primary_active)
        .add_modifier(Modifier::BOLD);

    let title_style = Style::default().fg(theme.ui.status_shortcut_fg);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        "Choose an editor",
        title_style,
    )]));
    lines.push(Line::from(""));

    if entries.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No editors found",
            Style::default().fg(theme.ui.status_error_fg),
        )]));
    } else {
        let has_terminal = entries.iter().any(|e| e.kind == EditorKind::Terminal);
        let has_gui = entries.iter().any(|e| e.kind == EditorKind::Gui);

        let mk_line = |entry: &crate::editor::EditorEntry, idx: usize| -> Line<'static> {
            let is_selected = idx == selected;
            let is_current = current_editor == Some(crate::editor::binary_name(&entry.command));
            let bg = if is_selected {
                theme.ui.toc_active_bg
            } else {
                theme.ui.toc_bg
            };
            let fg = if is_selected {
                theme.ui.toc_primary_active
            } else {
                theme.ui.toc_primary_inactive
            };
            let mut modifier = Modifier::empty();
            if is_selected || is_current {
                modifier |= Modifier::BOLD;
            }
            let marker = if is_selected { "▎ " } else { "  " };
            let check = if is_current { "  ✓" } else { "" };
            Line::from(vec![
                Span::styled(
                    marker.to_string(),
                    Style::default()
                        .fg(theme.ui.toc_accent)
                        .bg(bg)
                        .add_modifier(modifier),
                ),
                Span::styled(
                    entry.name.clone(),
                    Style::default().fg(fg).bg(bg).add_modifier(modifier),
                ),
                Span::styled(
                    check.to_string(),
                    Style::default()
                        .fg(theme.ui.toc_accent)
                        .bg(bg)
                        .add_modifier(modifier),
                ),
            ])
        };

        let mut item_idx = 0usize;
        if has_terminal {
            lines.push(Line::from(vec![Span::styled("Terminal", section_style)]));
            for entry in entries.iter().filter(|e| e.kind == EditorKind::Terminal) {
                lines.push(mk_line(entry, item_idx));
                item_idx += 1;
            }
        }
        if has_gui {
            if has_terminal {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(vec![Span::styled("GUI", section_style)]));
            for entry in entries.iter().filter(|e| e.kind == EditorKind::Gui) {
                lines.push(mk_line(entry, item_idx));
                item_idx += 1;
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(popup_footer_line(
        &["↑/↓ move", "enter confirm", "esc cancel"],
        theme.ui.toc_bg,
    ));

    let height = (lines.len() as u16 + 2).min(18);
    let area = centered_rect(42, height, f.area());

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title("─ Editor ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}
