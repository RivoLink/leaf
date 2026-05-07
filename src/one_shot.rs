use anyhow::Result;
use ratatui::{
    style::{Color, Modifier, Style},
    text::Line,
};
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorMode {
    Ansi,
    Plain,
}

pub(crate) fn terminal_render_width() -> usize {
    crossterm::terminal::size()
        .map(|(width, _)| usize::from(width).max(1))
        .unwrap_or(80)
}

pub(crate) fn write_lines<W: Write>(
    writer: &mut W,
    lines: &[Line<'_>],
    mode: ColorMode,
) -> Result<()> {
    let lines = trim_trailing_empty_lines(lines);
    for line in lines {
        for span in &line.spans {
            match mode {
                ColorMode::Ansi => {
                    write_style(writer, span.style)?;
                    write!(writer, "{}", span.content)?;
                    write!(writer, "\x1b[0m")?;
                }
                ColorMode::Plain => write!(writer, "{}", span.content)?,
            }
        }
        writeln!(writer)?;
    }
    Ok(())
}

fn trim_trailing_empty_lines<'a>(lines: &'a [Line<'a>]) -> &'a [Line<'a>] {
    let end = lines
        .iter()
        .rposition(|line| line.spans.iter().any(|span| !span.content.is_empty()))
        .map_or(0, |index| index + 1);
    &lines[..end]
}

fn write_style<W: Write>(writer: &mut W, style: Style) -> Result<()> {
    let mut codes = Vec::new();

    if let Some(color) = style.fg {
        push_color_code(&mut codes, color, false);
    }
    if let Some(color) = style.bg {
        push_color_code(&mut codes, color, true);
    }

    let modifiers = style.add_modifier;
    if modifiers.contains(Modifier::BOLD) {
        codes.push("1".to_string());
    }
    if modifiers.contains(Modifier::DIM) {
        codes.push("2".to_string());
    }
    if modifiers.contains(Modifier::ITALIC) {
        codes.push("3".to_string());
    }
    if modifiers.contains(Modifier::UNDERLINED) {
        codes.push("4".to_string());
    }
    if modifiers.contains(Modifier::CROSSED_OUT) {
        codes.push("9".to_string());
    }

    if !codes.is_empty() {
        write!(writer, "\x1b[{}m", codes.join(";"))?;
    }
    Ok(())
}

fn push_color_code(codes: &mut Vec<String>, color: Color, background: bool) {
    match color {
        Color::Reset => {}
        Color::Black => codes.push(base_code(background, 30).to_string()),
        Color::Red => codes.push(base_code(background, 31).to_string()),
        Color::Green => codes.push(base_code(background, 32).to_string()),
        Color::Yellow => codes.push(base_code(background, 33).to_string()),
        Color::Blue => codes.push(base_code(background, 34).to_string()),
        Color::Magenta => codes.push(base_code(background, 35).to_string()),
        Color::Cyan => codes.push(base_code(background, 36).to_string()),
        Color::Gray => codes.push(base_code(background, 37).to_string()),
        Color::DarkGray => codes.push(base_code(background, 90).to_string()),
        Color::LightRed => codes.push(base_code(background, 91).to_string()),
        Color::LightGreen => codes.push(base_code(background, 92).to_string()),
        Color::LightYellow => codes.push(base_code(background, 93).to_string()),
        Color::LightBlue => codes.push(base_code(background, 94).to_string()),
        Color::LightMagenta => codes.push(base_code(background, 95).to_string()),
        Color::LightCyan => codes.push(base_code(background, 96).to_string()),
        Color::White => codes.push(base_code(background, 97).to_string()),
        Color::Indexed(index) => {
            let prefix = if background { 48 } else { 38 };
            codes.push(format!("{prefix};5;{index}"));
        }
        Color::Rgb(red, green, blue) => {
            let prefix = if background { 48 } else { 38 };
            codes.push(format!("{prefix};2;{red};{green};{blue}"));
        }
    }
}

fn base_code(background: bool, code: u8) -> u8 {
    if background {
        code + 10
    } else {
        code
    }
}
