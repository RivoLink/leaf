use crate::{
    app::history::MSG_LOADING_FILE_HISTORY, app::App, editor::EditorKind, theme::app_theme,
};
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

const HISTORY_POPUP_TITLE: &str = "Open from file history";
const HISTORY_POPUP_BORDER: &str = "─ History ";
const HISTORY_POPUP_FOOTER: &[&str] = &["↑/↓ move", "<char> filter", "enter open", "esc close"];

fn picker_area(f: &Frame, app: &mut App, height: u16) -> Rect {
    let full = f.area();
    let width = app.refresh_picker_width_floor(full.width);
    centered_rect(width, height, full)
}

pub(super) fn render_file_popup(f: &mut Frame, app: &mut App) {
    let theme = app_theme();
    let area = picker_area(f, app, 20);
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
    let area = picker_area(f, app, 20);
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
        let query = app.file_picker_query();
        let (query_text, query_fg) = if query.is_empty() {
            (
                " type to filter ".to_string(),
                theme.ui.toc_primary_inactive,
            )
        } else {
            (format!(" {query} "), theme.ui.toc_primary_active)
        };
        lines.push(Line::from(vec![
            Span::styled("Query: ", section_style),
            Span::styled(
                query_text,
                Style::default()
                    .fg(query_fg)
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

fn truncate_middle(path: &str, max_width: usize) -> String {
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
    let total = UnicodeWidthStr::width(path);
    if total <= max_width || max_width <= 3 {
        return path.to_string();
    }
    let ellipsis = "...";
    let available = max_width.saturating_sub(3);
    let left_budget = available / 2;
    let right_budget = available - left_budget;

    let mut left = String::new();
    let mut left_width = 0usize;
    for ch in path.chars() {
        let w = ch.width().unwrap_or(0);
        if left_width + w > left_budget {
            break;
        }
        left.push(ch);
        left_width += w;
    }

    let mut right_stack: Vec<char> = Vec::new();
    let mut right_width = 0usize;
    for ch in path.chars().rev() {
        let w = ch.width().unwrap_or(0);
        if right_width + w > right_budget {
            break;
        }
        right_stack.push(ch);
        right_width += w;
    }
    let right: String = right_stack.into_iter().rev().collect();
    format!("{left}{ellipsis}{right}")
}

fn history_entry_spans(
    path: &std::path::Path,
    match_positions: &[usize],
    max_width: usize,
    bg: ratatui::style::Color,
    selected: bool,
    theme: &crate::theme::AppTheme,
) -> Vec<Span<'static>> {
    use std::path::MAIN_SEPARATOR;
    use unicode_width::UnicodeWidthStr;

    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let default_style = Style::default()
        .fg(theme.ui.toc_primary_inactive)
        .bg(bg)
        .add_modifier(if selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    if filename.is_empty() {
        return vec![Span::styled(
            truncate_middle(&path.to_string_lossy(), max_width),
            default_style,
        )];
    }

    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| {
            let mut s = p.to_string_lossy().into_owned();
            if !s.ends_with(MAIN_SEPARATOR) {
                s.push(MAIN_SEPARATOR);
            }
            s
        })
        .unwrap_or_default();

    let filename_width = UnicodeWidthStr::width(filename.as_str());
    let parent_width = UnicodeWidthStr::width(parent.as_str());

    if parent_width + filename_width <= max_width {
        let mut spans = vec![Span::styled(parent, default_style)];
        spans.extend(highlighted_picker_label(
            &filename,
            match_positions,
            bg,
            selected,
        ));
        return spans;
    }

    if filename_width + 4 <= max_width {
        let available = max_width - filename_width;
        let truncated_parent = truncate_middle(&parent, available);
        let mut spans = vec![Span::styled(truncated_parent, default_style)];
        spans.extend(highlighted_picker_label(
            &filename,
            match_positions,
            bg,
            selected,
        ));
        return spans;
    }

    vec![Span::styled(
        truncate_middle(&path.to_string_lossy(), max_width),
        default_style,
    )]
}

pub(super) fn render_history_popup(f: &mut Frame, app: &mut App) {
    let theme = app_theme();
    let area = picker_area(f, app, 17);
    let title_style = Style::default()
        .fg(theme.markdown.heading_2)
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default().fg(theme.ui.status_shortcut_fg);

    let activation_error = app.history_picker_activation_error().map(str::to_string);
    let inner_height = area.height.saturating_sub(2) as usize;
    let header_lines = 3;
    let reserved_footer_lines = if activation_error.is_some() { 3 } else { 2 };
    let visible_slots = inner_height
        .saturating_sub(header_lines + reserved_footer_lines)
        .min(10);
    let total = app.history_picker_filtered_indices().len();
    let start = if visible_slots == 0 || app.history_picker_index() < visible_slots {
        0
    } else {
        app.history_picker_index() + 1 - visible_slots
    };
    let end = (start + visible_slots).min(total);

    let mut lines = vec![Line::from(vec![Span::styled(
        HISTORY_POPUP_TITLE,
        title_style,
    )])];

    lines.push(Line::from(vec![
        Span::styled("Query: ", section_style),
        Span::styled(
            if app.history_picker_query().is_empty() {
                " type to filter ".to_string()
            } else {
                format!(" {} ", app.history_picker_query())
            },
            Style::default()
                .fg(if app.history_picker_query().is_empty() {
                    theme.ui.toc_primary_inactive
                } else {
                    theme.ui.toc_primary_active
                })
                .bg(theme.markdown.inline_code_bg),
        ),
    ]));

    lines.push(Line::from(""));

    let inner_width = area.width.saturating_sub(4) as usize;
    let label_max = inner_width.saturating_sub(2);

    if let Some(err) = app.history_picker_error() {
        lines.push(Line::from(vec![Span::styled(
            err.to_string(),
            Style::default().fg(theme.ui.toc_primary_inactive),
        )]));
    } else if total == 0 {
        lines.push(Line::from(vec![Span::styled(
            "No match for the current query",
            Style::default().fg(theme.ui.toc_primary_inactive),
        )]));
    } else {
        for (idx, entry_idx) in app.history_picker_filtered_indices()[start..end]
            .iter()
            .enumerate()
        {
            let actual_idx = start + idx;
            let selected = actual_idx == app.history_picker_index();
            let entry = &app.history_picker_entries()[*entry_idx];
            let bg = if selected {
                theme.ui.toc_active_bg
            } else {
                theme.ui.toc_bg
            };
            let marker = if selected { "▎ " } else { "  " };
            let modifier = if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            };
            let mut spans = vec![Span::styled(
                marker,
                Style::default()
                    .fg(theme.ui.toc_accent)
                    .bg(bg)
                    .add_modifier(modifier),
            )];
            spans.extend(history_entry_spans(
                &entry.path,
                app.history_picker_match_positions(actual_idx),
                label_max,
                bg,
                selected,
                &theme,
            ));
            lines.push(Line::from(spans));
        }
    }

    while lines.len() < inner_height.saturating_sub(reserved_footer_lines) {
        lines.push(Line::from(""));
    }

    if let Some(err) = activation_error {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            err,
            Style::default().fg(theme.markdown.heading_3),
        )]));
    } else {
        lines.push(Line::from(""));
    }
    lines.push(popup_footer_line(HISTORY_POPUP_FOOTER, theme.ui.toc_bg));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(HISTORY_POPUP_BORDER)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}

pub(super) fn render_history_loading_popup(f: &mut Frame, app: &mut App) {
    let theme = app_theme();
    let area = picker_area(f, app, 17);
    let title_style = Style::default()
        .fg(theme.markdown.heading_2)
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default().fg(theme.ui.status_shortcut_fg);

    let inner_height = area.height.saturating_sub(2) as usize;

    let mut lines = vec![
        Line::from(vec![Span::styled(HISTORY_POPUP_TITLE, title_style)]),
        Line::from(vec![
            Span::styled("Query: ", section_style),
            Span::styled(
                " type to filter ",
                Style::default()
                    .fg(theme.ui.toc_primary_inactive)
                    .bg(theme.markdown.inline_code_bg),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            MSG_LOADING_FILE_HISTORY,
            Style::default().fg(theme.ui.toc_primary_inactive),
        )]),
    ];

    while lines.len() < inner_height.saturating_sub(2) {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(""));
    lines.push(popup_footer_line(HISTORY_POPUP_FOOTER, theme.ui.toc_bg));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(HISTORY_POPUP_BORDER)
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
