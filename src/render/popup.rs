use crate::{
    app::{App, PathKind, FLASH_DURATION_MS},
    cli::version_text,
    theme::{app_theme, theme_preset_label, THEME_PRESETS},
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};
use std::time::Duration;

use super::centered_rect;

const FUZZY_PICKER_FOOTER_INIT: &[&str] = &[
    "↑/↓ move",
    "<char> filter",
    "enter open",
    "esc clear",
    "ctrl+c quit",
];

const FUZZY_PICKER_FOOTER_PREVIEW: &[&str] = &[
    "↑/↓ move",
    "<char> filter",
    "enter open",
    "esc clear",
    "ctrl+c close",
];

const BROWSER_PICKER_FOOTER_INIT: &[&str] = &["↑/↓ move", "enter open", "bsp parent", "q quit"];

const BROWSER_PICKER_FOOTER_PREVIEW: &[&str] =
    &["↑/↓ move", "enter open", "bsp parent", "ctrl+c close"];

const PICKER_FAILED_FOOTER_INIT: &[&str] = &["esc quit", "enter quit", "q quit"];

const PICKER_FAILED_FOOTER_PREVIEW: &[&str] = &["esc close", "enter close", "ctrl+c close"];

pub(super) fn popup_footer(
    has_content: bool,
    is_fuzzy: bool,
    is_failed: bool,
) -> &'static [&'static str] {
    if has_content {
        if is_failed {
            PICKER_FAILED_FOOTER_PREVIEW
        } else if is_fuzzy {
            FUZZY_PICKER_FOOTER_PREVIEW
        } else {
            BROWSER_PICKER_FOOTER_PREVIEW
        }
    } else if is_failed {
        PICKER_FAILED_FOOTER_INIT
    } else if is_fuzzy {
        FUZZY_PICKER_FOOTER_INIT
    } else {
        BROWSER_PICKER_FOOTER_INIT
    }
}

pub(super) fn popup_footer_line(segments: &[&'static str], bg: Color) -> Line<'static> {
    let theme = app_theme();
    let shortcut_style = Style::default().fg(theme.ui.status_shortcut_fg).bg(bg);
    let separator_style = Style::default().fg(theme.ui.status_separator).bg(bg);
    let mut spans = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(" · ", separator_style));
        }
        spans.push(Span::styled(*segment, shortcut_style));
    }
    Line::from(spans)
}

pub(super) fn render_help_popup(f: &mut Frame, _app: &App) {
    let theme = app_theme();
    let area = centered_rect(53, 26, f.area());

    let select_hint =
        crate::editor::selection_modifier_label(&crate::editor::detect_terminal_emulator());
    let section_style = Style::default()
        .fg(theme.ui.toc_primary_active)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(theme.ui.toc_accent)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(theme.ui.toc_primary_inactive);

    let title_style = Style::default()
        .fg(theme.markdown.heading_2)
        .add_modifier(Modifier::BOLD);
    let lines = vec![
        Line::from(vec![Span::styled(version_text().to_string(), title_style)]),
        Line::from(vec![Span::styled(
            "Keyboard shortcuts",
            Style::default().fg(theme.ui.status_shortcut_fg),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation                  Mouse",
            section_style,
        )]),
        Line::from(vec![
            Span::styled("j/k, ↑/↓   ", key_style),
            Span::styled("scroll", text_style),
            Span::raw("           "),
            Span::styled("shift+m     ", key_style),
            Span::styled("capture", text_style),
        ]),
        Line::from(vec![
            Span::styled("u/d        ", key_style),
            Span::styled("page up/down", text_style),
            Span::raw("     "),
            Span::styled("dbl-click   ", key_style),
            Span::styled("copy code", text_style),
        ]),
        Line::from(vec![
            Span::styled("g/G        ", key_style),
            Span::styled("top/bottom", text_style),
            Span::raw("       "),
            Span::styled("dbl-click   ", key_style),
            Span::styled("copy link", text_style),
        ]),
        Line::from(vec![
            Span::styled("1-9/0+1-9  ", key_style),
            Span::styled("jump/reverse", text_style),
            Span::raw("     "),
            Span::styled("ctrl+click  ", key_style),
            Span::styled("open link", text_style),
        ]),
        Line::from(vec![
            Span::styled("y/Y, c/C   ", key_style),
            Span::styled("focus code", text_style),
            Span::raw("       "),
            Span::styled(format!("{select_hint:<12}"), key_style),
            Span::styled("slct text", text_style),
        ]),
        Line::from(vec![
            Span::styled("J/K, U/D   ", key_style),
            Span::styled("navigate toc", text_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Search                      ", section_style),
            Span::styled("Watch", section_style),
        ]),
        Line::from(vec![
            Span::styled("ctrl+f     ", key_style),
            Span::styled("find", text_style),
            Span::raw("             "),
            Span::styled("ctrl+w, w   ", key_style),
            Span::styled("toggle", text_style),
        ]),
        Line::from(vec![
            Span::styled("n/N        ", key_style),
            Span::styled("next/prev", text_style),
            Span::raw("        "),
            Span::styled("ctrl+r, r   ", key_style),
            Span::styled("reload", text_style),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Actions", section_style)]),
        Line::from(vec![
            Span::styled("shift+e    ", key_style),
            Span::styled("editor picker", text_style),
            Span::raw("    "),
            Span::styled("ctrl+e      ", key_style),
            Span::styled("edit", text_style),
        ]),
        Line::from(vec![
            Span::styled("shift+l    ", key_style),
            Span::styled("line number", text_style),
            Span::raw("      "),
            Span::styled("ctrl+l      ", key_style),
            Span::styled("goto", text_style),
        ]),
        Line::from(vec![
            Span::styled("shift+p    ", key_style),
            Span::styled("file browser", text_style),
            Span::raw("     "),
            Span::styled("ctrl+p      ", key_style),
            Span::styled("pick", text_style),
        ]),
        Line::from(vec![
            Span::styled("shift+t    ", key_style),
            Span::styled("theme picker", text_style),
            Span::raw("     "),
            Span::styled("?           ", key_style),
            Span::styled("help", text_style),
        ]),
        Line::from(vec![
            Span::styled("p          ", key_style),
            Span::styled("path viewer", text_style),
            Span::raw("      "),
            Span::styled("q           ", key_style),
            Span::styled("quit", text_style),
        ]),
        Line::from(vec![
            Span::styled("t          ", key_style),
            Span::styled("toggle toc", text_style),
        ]),
        Line::from(""),
        popup_footer_line(&["esc close", "? close"], theme.ui.toc_bg),
    ];

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title("─ Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}

pub(super) fn render_theme_popup(f: &mut Frame, app: &App) {
    let theme = app_theme();
    let area = centered_rect(43, 10, f.area());
    let active = app.theme_picker_reference_preset();

    let mut lines = vec![
        Line::from(vec![Span::styled(
            "Choose a theme",
            Style::default().fg(theme.ui.status_shortcut_fg),
        )]),
        Line::from(""),
    ];
    for (idx, preset) in THEME_PRESETS.iter().enumerate() {
        let selected = idx == app.theme_picker_index();
        let is_active = active == Some(*preset);
        let bg = if selected {
            theme.ui.toc_active_bg
        } else {
            theme.ui.toc_bg
        };
        let marker = if selected { "▎ " } else { "  " };
        let check = if is_active { "  ✓" } else { "" };
        let modifier = if is_active || selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        };
        lines.push(Line::from(vec![
            Span::styled(
                marker,
                Style::default()
                    .fg(theme.ui.toc_accent)
                    .bg(bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::styled(
                theme_preset_label(*preset),
                Style::default()
                    .fg(if selected {
                        theme.ui.toc_primary_active
                    } else {
                        theme.ui.toc_primary_inactive
                    })
                    .bg(bg)
                    .add_modifier(modifier),
            ),
            Span::styled(
                check,
                Style::default()
                    .fg(theme.ui.toc_accent)
                    .bg(bg)
                    .add_modifier(modifier),
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(popup_footer_line(
        &["↑/↓ preview", "enter keep", "esc restore"],
        theme.ui.toc_bg,
    ));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title("─ Theme ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}

pub(super) fn picker_truncation_message(
    truncation: Option<crate::app::PickerIndexTruncation>,
) -> Option<&'static str> {
    match truncation {
        Some(crate::app::PickerIndexTruncation::Directory) => {
            Some("Indexing limited: directory limit reached")
        }
        Some(crate::app::PickerIndexTruncation::File) => {
            Some("Indexing limited: file limit reached")
        }
        Some(crate::app::PickerIndexTruncation::Time) => {
            Some("Indexing limited: time limit reached")
        }
        None => None,
    }
}

pub(super) fn highlighted_picker_label(
    label: &str,
    match_positions: &[usize],
    bg: Color,
    selected: bool,
) -> Vec<Span<'static>> {
    let theme = app_theme();
    let default_style = Style::default()
        .fg(theme.ui.toc_primary_inactive)
        .bg(bg)
        .add_modifier(if selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    let matched_style = Style::default()
        .fg(theme.ui.toc_accent)
        .bg(bg)
        .add_modifier(if selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    if match_positions.is_empty() {
        return vec![Span::styled(label.to_string(), default_style)];
    }

    let match_set = match_positions
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut spans = Vec::new();
    let mut buffer = String::new();
    let mut current_matched = None;

    for (idx, ch) in label.chars().enumerate() {
        let is_matched = match_set.contains(&idx);
        if current_matched == Some(is_matched) || current_matched.is_none() {
            buffer.push(ch);
            current_matched = Some(is_matched);
            continue;
        }

        spans.push(Span::styled(
            std::mem::take(&mut buffer),
            if current_matched == Some(true) {
                matched_style
            } else {
                default_style
            },
        ));
        buffer.push(ch);
        current_matched = Some(is_matched);
    }

    if !buffer.is_empty() {
        spans.push(Span::styled(
            buffer,
            if current_matched == Some(true) {
                matched_style
            } else {
                default_style
            },
        ));
    }

    spans
}

pub(crate) fn wrap_path_lines(
    label: &str,
    path: &str,
    max_width: usize,
    label_style: Style,
    value_style: Style,
) -> Vec<Line<'static>> {
    let label_len = label.len();
    let value_width = max_width.saturating_sub(label_len);
    if path.len() <= value_width {
        return vec![Line::from(vec![
            Span::styled(label.to_string(), label_style),
            Span::styled(path.to_string(), value_style),
        ])];
    }
    let indent = " ".repeat(label_len);
    let mut result = Vec::new();
    let mut pos = 0;
    while pos < path.len() {
        let end = (pos + value_width).min(path.len());
        if pos == 0 {
            result.push(Line::from(vec![
                Span::styled(label.to_string(), label_style),
                Span::styled(path[..end].to_string(), value_style),
            ]));
        } else {
            result.push(Line::from(vec![
                Span::raw(indent.clone()),
                Span::styled(path[pos..end].to_string(), value_style),
            ]));
        }
        pos = end;
    }
    result
}

pub(super) fn render_path_popup(f: &mut Frame, app: &mut App) {
    let theme = app_theme();
    let label_style = Style::default()
        .fg(theme.ui.toc_accent)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(theme.ui.toc_primary_inactive);

    let relative = app.relative_path_string();
    let absolute = app.absolute_path_string();

    const POPUP_WIDTH: usize = 78;
    const CHROME: usize = 4; // 2 borders + 2 padding
    let max_width = POPUP_WIDTH - CHROME;
    let mut rel_lines =
        wrap_path_lines("Relative: ", &relative, max_width, label_style, value_style);
    let mut abs_lines =
        wrap_path_lines("Absolute: ", &absolute, max_width, label_style, value_style);

    let flash_info = app
        .path_copy_flash()
        .and_then(|(target, success, started)| {
            if started.elapsed() < Duration::from_millis(FLASH_DURATION_MS) {
                Some((target.clone(), *success))
            } else {
                None
            }
        });

    if let Some((ref target, success)) = flash_info {
        let (label, fg) = if success {
            (" ✓ copied", theme.ui.status_success_fg)
        } else {
            (" ✗ error", theme.ui.status_error_fg)
        };
        let style = Style::default().fg(fg).bg(theme.ui.toc_bg);
        let lines_to_tag = match target {
            PathKind::Relative => &mut rel_lines,
            PathKind::Absolute => &mut abs_lines,
        };
        if let Some(first_line) = lines_to_tag.first_mut() {
            first_line.spans.push(Span::styled(label, style));
        }
    }

    let hover_lines = match app.path_popup_hover {
        Some(PathKind::Relative) => Some(&mut rel_lines),
        Some(PathKind::Absolute) => Some(&mut abs_lines),
        None => None,
    };
    if let Some(lines) = hover_lines {
        for line in lines {
            for span in &mut line.spans {
                if span.style == value_style {
                    span.style = span.style.fg(theme.markdown.link_hover);
                }
            }
        }
    }

    let content_height = 1 + rel_lines.len() + abs_lines.len() + 1 + 1;
    let popup_height = content_height + 2;

    let area = centered_rect(POPUP_WIDTH as u16, popup_height as u16, f.area());

    let inner_x = area.x + 1 + 1; // border + padding
    let inner_width = area.width.saturating_sub(4); // 2 borders + 2 padding
    let rel_start_y = area.y + 1 + 1; // border + top pad
    let rel_count = rel_lines.len() as u16;
    let abs_start_y = rel_start_y + rel_count;
    let abs_count = abs_lines.len() as u16;

    app.path_popup_rel_area = Some(Rect::new(inner_x, rel_start_y, inner_width, rel_count));
    app.path_popup_abs_area = Some(Rect::new(inner_x, abs_start_y, inner_width, abs_count));

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(""));
    lines.extend(rel_lines);
    lines.extend(abs_lines);
    lines.push(Line::from(""));
    lines.push(popup_footer_line(
        &[
            "shift+r copy-relative",
            "shift+a copy-absolute",
            "esc close",
        ],
        theme.ui.toc_bg,
    ));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title("─ File Path ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.toc_border))
                .style(Style::default().bg(theme.ui.toc_bg))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}
