use crate::{
    app::{App, CodeBlockFlash, EditorFlash, LinkFlash, PathFlash, WatchFlash, FLASH_DURATION_MS},
    theme::app_theme,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

pub(crate) fn status_bar_bg() -> Color {
    app_theme().ui.status_bg
}

pub(crate) fn status_separator_style(bar_bg: Color) -> Style {
    Style::default()
        .fg(app_theme().ui.status_separator)
        .bg(bar_bg)
}

pub(crate) fn join_span_sections(
    sections: Vec<Vec<Span<'static>>>,
    separator: Span<'static>,
) -> Vec<Span<'static>> {
    let mut joined = Vec::new();
    for (idx, section) in sections.into_iter().enumerate() {
        if idx > 0 {
            joined.push(separator.clone());
        }
        joined.extend(section);
    }
    joined
}

pub(crate) fn status_brand_section() -> Vec<Span<'static>> {
    let theme = app_theme();
    vec![Span::styled(
        " leaf ",
        Style::default()
            .fg(theme.ui.status_brand_fg)
            .bg(theme.ui.status_brand_bg)
            .add_modifier(Modifier::BOLD),
    )]
}

pub(crate) fn status_filename_section(filename: &str) -> Vec<Span<'static>> {
    let theme = app_theme();
    vec![Span::styled(
        format!(" {} ", filename),
        Style::default()
            .fg(theme.ui.status_filename_fg)
            .bg(theme.ui.status_filename_bg),
    )]
}

fn watch_flash_section(app: &App) -> Option<Vec<Span<'static>>> {
    let (flash, started) = app.watch_flash()?;
    if started.elapsed() >= std::time::Duration::from_millis(FLASH_DURATION_MS) {
        return None;
    }
    let theme = app_theme();
    let bar_bg = status_bar_bg();
    let (text, fg) = match flash {
        WatchFlash::Activated => (" Watch mode activated ", theme.ui.status_success_fg),
        WatchFlash::Deactivated => (" Watch mode deactivated ", theme.ui.status_warning_fg),
        WatchFlash::Stdin => (" Stdin cannot be watched ", theme.ui.status_error_fg),
        WatchFlash::NoFile => (" No file to watch ", theme.ui.status_error_fg),
        WatchFlash::FileNotFound => (" File not found ", theme.ui.status_error_fg),
        WatchFlash::NotActive => (" Watch mode is not active ", theme.ui.status_warning_fg),
    };
    Some(vec![Span::styled(text, Style::default().fg(fg).bg(bar_bg))])
}

pub(crate) fn status_watch_section(app: &App) -> Option<Vec<Span<'static>>> {
    let theme = app_theme();
    if !app.is_watch_enabled() {
        return None;
    }

    if app.is_watch_error() {
        return Some(vec![Span::styled(
            " ⟳ error ",
            Style::default()
                .fg(theme.ui.status_error_fg)
                .bg(theme.ui.status_error_bg),
        )]);
    }

    let flash_active = app
        .reload_flash_started()
        .map(|t| t.elapsed() < std::time::Duration::from_millis(FLASH_DURATION_MS))
        .unwrap_or(false);
    let span = if flash_active {
        Span::styled(
            " ⟳ reloaded ",
            Style::default()
                .fg(theme.ui.status_reloaded_fg)
                .bg(theme.ui.status_reloaded_bg),
        )
    } else {
        Span::styled(
            " ⟳ watch ",
            Style::default()
                .fg(theme.ui.status_watch_fg)
                .bg(theme.ui.status_watch_bg),
        )
    };
    Some(vec![span])
}

pub(crate) fn status_search_section(app: &App) -> Option<Vec<Span<'static>>> {
    let theme = app_theme();
    if app.is_search_mode() {
        return Some(vec![Span::styled(
            format!(" /{} ", app.search_draft()),
            Style::default()
                .fg(theme.ui.status_search_fg)
                .bg(theme.ui.status_search_bg),
        )]);
    }

    if app.search_query().is_empty() {
        return None;
    }

    let span = if app.search_match_count() == 0 {
        Span::styled(
            format!(" ✗ {} ", app.search_query()),
            Style::default()
                .fg(theme.ui.status_error_fg)
                .bg(theme.ui.status_error_bg),
        )
    } else {
        Span::styled(
            format!(" {}/{} ", app.search_index() + 1, app.search_match_count()),
            Style::default()
                .fg(theme.ui.status_success_fg)
                .bg(theme.ui.status_success_bg),
        )
    };
    Some(vec![span])
}

pub(crate) fn status_goto_line_section(app: &App) -> Option<Vec<Span<'static>>> {
    let theme = app_theme();
    if app.is_goto_line_mode() {
        return Some(vec![Span::styled(
            format!(" :{} ", app.goto_line_draft()),
            Style::default()
                .fg(theme.ui.status_search_fg)
                .bg(theme.ui.status_search_bg),
        )]);
    }

    if !app.has_active_goto_line() {
        return None;
    }

    let span = if app.goto_line_error() {
        Span::styled(
            format!(" ✗ :{} ", app.goto_line_draft()),
            Style::default()
                .fg(theme.ui.status_error_fg)
                .bg(theme.ui.status_error_bg),
        )
    } else if let Some(target) = app.goto_line_target() {
        let logical = app.line_number_at(target);
        Span::styled(
            format!(" :{} ", logical),
            Style::default()
                .fg(theme.ui.status_success_fg)
                .bg(theme.ui.status_success_bg),
        )
    } else {
        return None;
    };
    Some(vec![span])
}

pub(crate) fn status_hint_segments(app: &App) -> &'static [&'static str] {
    if app.is_goto_line_mode() || app.is_search_mode() {
        &["enter confirm", "esc cancel"]
    } else if app.has_active_goto_line() {
        &["esc cancel"]
    } else if app.has_active_search() {
        &["n/N next/prev", "esc cancel"]
    } else {
        &["ctrl+e edit", "ctrl+f find", "t toc", "? help", "q quit"]
    }
}

pub(crate) fn status_shortcuts_section(app: &App, bar_bg: Color) -> Vec<Span<'static>> {
    let theme = app_theme();
    let separator = Span::styled(" · ", status_separator_style(bar_bg));
    let sections = status_hint_segments(app)
        .iter()
        .map(|segment| {
            vec![Span::styled(
                *segment,
                Style::default().fg(theme.ui.status_shortcut_fg).bg(bar_bg),
            )]
        })
        .collect();
    join_span_sections(sections, separator)
}

pub(crate) fn status_percent_section(pct: u16, bar_bg: Color) -> Vec<Span<'static>> {
    let theme = app_theme();
    vec![Span::styled(
        format!("{:>3}% ", pct),
        Style::default().fg(theme.ui.status_percent_fg).bg(bar_bg),
    )]
}

fn config_flash_section(app: &App) -> Option<Vec<Span<'static>>> {
    let (message, started) = app.config_flash()?;
    if started.elapsed() >= std::time::Duration::from_millis(FLASH_DURATION_MS) {
        return None;
    }
    let theme = app_theme();
    let bar_bg = status_bar_bg();
    Some(vec![Span::styled(
        format!(" {message} "),
        Style::default().fg(theme.ui.status_warning_fg).bg(bar_bg),
    )])
}

fn editor_flash_section(app: &App) -> Option<Vec<Span<'static>>> {
    let (flash, started) = app.editor_flash()?;
    if started.elapsed() >= std::time::Duration::from_millis(FLASH_DURATION_MS) {
        return None;
    }
    let theme = app_theme();
    let bar_bg = status_bar_bg();
    let (message, fg) = match flash {
        EditorFlash::Opened(name) => (format!(" Opened in {name} "), theme.ui.status_success_fg),
        EditorFlash::NoFile => (" No file to edit ".to_string(), theme.ui.status_error_fg),
        EditorFlash::EditorNotFound(msg) => (
            format!(" Editor not found: {msg} "),
            theme.ui.status_error_fg,
        ),
    };
    Some(vec![Span::styled(
        message,
        Style::default().fg(fg).bg(bar_bg),
    )])
}

fn clipboard_hint() -> &'static str {
    match std::env::consts::OS {
        "macos" => " Copy failed: pbcopy not found ",
        "windows" => " Copy failed: clip.exe not found ",
        _ => " Copy failed: install xclip or wl-clipboard ",
    }
}

fn link_flash_section(app: &App) -> Option<Vec<Span<'static>>> {
    let (flash, started) = app.link_flash()?;
    if started.elapsed() >= std::time::Duration::from_millis(FLASH_DURATION_MS) {
        return None;
    }
    let theme = app_theme();
    let bar_bg = status_bar_bg();
    let (text, fg) = match flash {
        LinkFlash::Copied => (" Copied to clipboard ", theme.ui.status_success_fg),
        LinkFlash::CopyFailed => (clipboard_hint(), theme.ui.status_error_fg),
    };
    Some(vec![Span::styled(text, Style::default().fg(fg).bg(bar_bg))])
}

fn code_block_flash_section(app: &App) -> Option<Vec<Span<'static>>> {
    let (flash, started) = app.code_block_flash()?;
    if started.elapsed() >= std::time::Duration::from_millis(FLASH_DURATION_MS) {
        return None;
    }
    let theme = app_theme();
    let bar_bg = status_bar_bg();
    let (text, fg) = match flash {
        CodeBlockFlash::Copied => (" Code copied ", theme.ui.status_success_fg),
        CodeBlockFlash::CopyFailed => (clipboard_hint(), theme.ui.status_error_fg),
        CodeBlockFlash::NoneVisible => (" No code block in view ", theme.ui.status_error_fg),
    };
    Some(vec![Span::styled(text, Style::default().fg(fg).bg(bar_bg))])
}

fn path_flash_section(app: &App) -> Option<Vec<Span<'static>>> {
    let (flash, started) = app.path_flash()?;
    if started.elapsed() >= std::time::Duration::from_millis(FLASH_DURATION_MS) {
        return None;
    }
    let theme = app_theme();
    let bar_bg = status_bar_bg();
    let (text, fg) = match flash {
        PathFlash::RelativeCopied => (
            " Relative path copied to clipboard ",
            theme.ui.status_success_fg,
        ),
        PathFlash::AbsoluteCopied => (
            " Absolute path copied to clipboard ",
            theme.ui.status_success_fg,
        ),
        PathFlash::CopyFailed => (clipboard_hint(), theme.ui.status_error_fg),
    };
    Some(vec![Span::styled(text, Style::default().fg(fg).bg(bar_bg))])
}

pub(crate) fn build_status_bar(app: &App, pct: u16) -> Vec<Span<'static>> {
    let bar_bg = status_bar_bg();
    let outer_separator = Span::raw(" ");

    if !app.is_mouse_capture_enabled() {
        let theme = app_theme();
        let mut left = status_brand_section();
        left.push(Span::styled(
            " Mouse capture disabled ",
            Style::default().fg(theme.ui.status_warning_fg).bg(bar_bg),
        ));
        return join_span_sections(vec![left], outer_separator);
    }

    if let Some(flash_section) = editor_flash_section(app) {
        let mut left = status_brand_section();
        left.extend(flash_section);
        return join_span_sections(vec![left], outer_separator);
    }

    if let Some(flash_section) = watch_flash_section(app) {
        let mut left = status_brand_section();
        left.extend(flash_section);
        return join_span_sections(vec![left], outer_separator);
    }

    if let Some(flash_section) = config_flash_section(app) {
        let mut left = status_brand_section();
        left.extend(flash_section);
        return join_span_sections(vec![left], outer_separator);
    }

    if let Some(flash_section) = link_flash_section(app) {
        let mut left = status_brand_section();
        left.extend(flash_section);
        return join_span_sections(vec![left], outer_separator);
    }

    if let Some(flash_section) = code_block_flash_section(app) {
        let mut left = status_brand_section();
        left.extend(flash_section);
        return join_span_sections(vec![left], outer_separator);
    }

    if let Some(flash_section) = path_flash_section(app) {
        let mut left = status_brand_section();
        left.extend(flash_section);
        return join_span_sections(vec![left], outer_separator);
    }

    if app.is_toc_scroll_hint_visible() {
        let theme = app_theme();
        let mut left = status_brand_section();
        left.push(Span::styled(
            " Navigate with shift+j/k and shift+u/d ",
            Style::default().fg(theme.ui.status_warning_fg).bg(bar_bg),
        ));
        return join_span_sections(vec![left], outer_separator);
    }

    let mut left_section = status_brand_section();
    left_section.extend(status_filename_section(app.filename()));

    if let Some(section) = status_search_section(app) {
        left_section.extend(section);
    }

    if let Some(section) = status_goto_line_section(app) {
        left_section.extend(section);
    }

    let file_open = app.has_content() || (!app.is_file_picker_open() && !app.is_picker_loading());
    if file_open {
        if let Some(section) = status_watch_section(app) {
            left_section.extend(section);
        }
    }

    let mut sections = vec![left_section, status_shortcuts_section(app, bar_bg)];
    if file_open {
        sections.push(status_percent_section(pct, bar_bg));
    }

    join_span_sections(sections, outer_separator)
}
