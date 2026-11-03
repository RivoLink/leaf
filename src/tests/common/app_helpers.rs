use super::super::{test_assets, test_md_theme, test_syntax_set, test_theme_set};
use crate::app::{App, AppConfig};
use crate::markdown::{parse_markdown_with_width, ParseResult};
use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

pub(crate) fn parse_markdown_for_test(source: &str, parse_width: usize) -> ParseResult {
    let (ss, theme) = test_assets();
    parse_markdown_with_width(
        source,
        &ss,
        &theme,
        parse_width,
        &test_md_theme(),
        false,
        true,
    )
}

pub(crate) fn markdown_app(
    source: &str,
    parse_width: usize,
    filename: &str,
    filepath: Option<&str>,
) -> App {
    let parsed = parse_markdown_for_test(source, parse_width);
    let mut app = App::new_with_source(
        Vec::new(),
        Vec::new(),
        AppConfig {
            filename: filename.into(),
            source: source.into(),
            debug_input: false,
            watch: false,
            filepath: filepath.map(Into::into),
            last_file_state: None,
        },
    );
    app.replace_content(parsed);
    app
}

pub(crate) fn draw_app(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| crate::render::ui(frame, app))
        .unwrap();
    terminal.backend().buffer().clone()
}

pub(crate) fn sync_app_width(app: &mut App, width: u16) {
    crate::runtime::sync_render_width_for_app(width, app, &test_syntax_set(), test_theme_set());
}

pub(crate) fn status_text(buffer: &Buffer) -> String {
    let mut text = String::new();
    let mut x = 0;
    while x < buffer.area.width {
        let symbol = buffer[(x, buffer.area.height - 1)].symbol();
        text.push_str(symbol);
        x += ratatui::text::Span::raw(symbol).width().max(1) as u16;
    }
    text
}
