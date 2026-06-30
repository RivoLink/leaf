use crate::*;
use ratatui::backend::TestBackend;
use ratatui::{text::Line, widgets::Paragraph, Terminal};
use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};
use syntect::{
    highlighting::{Theme, ThemeSet},
    parsing::SyntaxSet,
};

mod app;
mod app_code_blocks;
mod completions;
mod config;
mod editor;
mod file_fuzzy;
mod file_picker;
mod inline;
mod markdown_blocks;
mod markdown_embedded;
mod markdown_links;
mod markdown_lists;
mod markdown_tables;
mod render;
mod theme;
mod toc;
mod update;

pub(super) static THEME_TEST_MUTEX: Mutex<()> = Mutex::new(());

pub(super) fn test_assets() -> (SyntaxSet, Theme) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = ts.themes["base16-ocean.dark"].clone();
    (ss, theme)
}

pub(super) fn test_md_theme() -> crate::theme::MarkdownTheme {
    crate::theme::app_theme().markdown
}

pub(super) fn render_buffer(lines: &[Line<'static>]) -> ratatui::buffer::Buffer {
    let width = lines
        .iter()
        .map(|line| line.width())
        .max()
        .unwrap_or(1)
        .max(1) as u16;
    let height = lines.len().max(1) as u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            f.render_widget(Paragraph::new(lines.to_vec()), f.area());
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

pub(super) fn find_symbol(buffer: &ratatui::buffer::Buffer, symbol: &str) -> Option<(u16, u16)> {
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer
                .cell((x, y))
                .is_some_and(|cell| cell.symbol() == symbol)
            {
                return Some((x, y));
            }
        }
    }
    None
}

pub(super) fn rendered_non_empty_lines(lines: &[Line<'static>]) -> Vec<String> {
    lines
        .iter()
        .map(line_plain_text)
        .filter(|line| !line.is_empty())
        .collect()
}

pub(super) fn lock_theme_test_state() -> MutexGuard<'static, ()> {
    THEME_TEST_MUTEX.lock().unwrap()
}

pub(super) fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{unique}"))
}
