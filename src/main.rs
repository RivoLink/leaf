use anyhow::{bail, Context, Result};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{fs::OpenOptions, io, io::IsTerminal, io::Read, io::Write, path::PathBuf};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

mod app;
mod cli;
mod config;
mod editor;
mod markdown;
mod one_shot;
mod render;
mod runtime;
mod terminal;
#[cfg(test)]
mod tests;
mod theme;
mod update;

use app::{App, AppConfig};
use cli::{parse_cli, print_usage, print_version, CliOptions};
use markdown::{hash_str, parse_markdown, parse_markdown_with_width, read_file_state};
use one_shot::{terminal_render_width, write_lines, ColorMode};
use runtime::run;
use terminal::{finish_with_restore, TerminalSession};
use theme::{
    app_theme, current_syntect_theme, resolve_theme_selection, set_theme_selection,
    validate_theme_syntax,
};
use update::run_update;

const MAX_STDIN_BYTES: usize = 8 * 1024 * 1024;

#[cfg(test)]
pub(crate) use config::{config_path, LeafConfig};
#[cfg(test)]
pub(crate) use editor::{binary_name, classify, resolve_editor, split_editor_cmd, EditorKind};
#[cfg(test)]
pub(crate) use markdown::toc::{
    normalize_toc, should_hide_single_h1, should_promote_h2_when_no_h1, toc_display_level, TocEntry,
};
#[cfg(test)]
pub(crate) use markdown::{display_width, line_plain_text};
#[cfg(test)]
pub(crate) use read_stdin_limited as read_stdin_with_limit;
#[cfg(test)]
pub(crate) use render::wrap_path_lines;
#[cfg(test)]
pub(crate) use runtime::should_handle_key;
#[cfg(test)]
pub(crate) use theme::{
    parse_theme_color, parse_theme_preset, theme_preset_label, CustomThemeConfig, ThemePreset,
    ThemeSelection, THEME_PRESETS,
};
#[cfg(test)]
pub(crate) use update::{
    asset_name_for_target, expected_asset_download_url, find_expected_checksum, is_newer_version,
    validate_download_size, validate_sha256_hex,
};

fn read_stdin_limited<R: Read>(reader: &mut R, max_bytes: usize) -> Result<String> {
    let mut buf = Vec::with_capacity(max_bytes.min(8192));
    let limit = u64::try_from(max_bytes)
        .ok()
        .and_then(|value| value.checked_add(1))
        .context("stdin size limit is too large")?;
    reader
        .take(limit)
        .read_to_end(&mut buf)
        .context("Cannot read stdin")?;
    if buf.len() > max_bytes {
        bail!(
            "stdin exceeds the maximum supported size of {} bytes",
            max_bytes
        );
    }
    String::from_utf8(buf).context("stdin is not valid UTF-8")
}

fn append_config_warning(warning: &mut Option<String>, next: Option<String>) {
    let Some(next) = next else {
        return;
    };
    match warning {
        Some(existing) => {
            existing.push_str("; ");
            existing.push_str(&next);
        }
        None => *warning = Some(next),
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let options = parse_cli(&args)?;

    if options.print_help {
        print_usage();
        return Ok(());
    }
    if options.print_version {
        print_version();
        return Ok(());
    }
    if options.update {
        run_update()?;
        return Ok(());
    }
    if options.config {
        config::run_config()?;
        return Ok(());
    }
    let CliOptions {
        picker,
        watch: watch_from_cli,
        render,
        ansi,
        plain,
        debug_input,
        file_arg,
        theme: cli_theme,
        editor: cli_editor,
        ..
    } = options;

    let (user_config, mut config_warning) = config::load_config();

    let theme_selection = if let Some(theme_name) = cli_theme.as_deref() {
        resolve_theme_selection(theme_name, &user_config.themes, None)
            .map_err(|message| anyhow::anyhow!("{message}"))?
    } else if let Some(theme_name) = user_config.theme.as_deref() {
        resolve_theme_selection(
            theme_name,
            &user_config.themes,
            user_config.config_dir.as_deref(),
        )
        .unwrap_or_default()
    } else {
        theme::ThemeSelection::default()
    };

    let watch_from_config = user_config.watch.unwrap_or(false);

    let resolved_editor =
        editor::resolve_editor(cli_editor.as_deref(), user_config.editor.as_deref());
    runtime::debug_log(debug_input, &format!("main start args={args:?}"));

    if debug_input {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("leaf-debug.log")
            .context("Cannot create leaf-debug.log")?;
        writeln!(file, "leaf debug input log").ok();
    }

    let mut open_browser_picker_dir = None;
    let mut open_fuzzy_picker_dir = None;
    let (src, filename, filepath) = if let Some(f) = file_arg {
        let path = PathBuf::from(&f);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Cannot read: {}", path.display()))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(f.clone());
        (content, name, Some(path))
    } else {
        if io::stdin().is_terminal() {
            if render {
                bail!("--render requires a file path or stdin");
            }
            let cwd = std::env::current_dir().context("Cannot read current directory")?;
            let label = cwd
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| cwd.display().to_string());
            if picker {
                open_browser_picker_dir = Some(cwd);
            } else {
                open_fuzzy_picker_dir = Some(cwd);
            }
            (String::new(), label, None)
        } else {
            if watch_from_cli {
                eprintln!("Error: --watch requires a file path (stdin cannot be watched)");
                std::process::exit(1);
            }
            let mut stdin = io::stdin().lock();
            let buf = read_stdin_limited(&mut stdin, MAX_STDIN_BYTES)?;
            (buf, "stdin".to_string(), None)
        }
    };

    let is_file_input = filepath.is_some();
    let watch = watch_from_cli || (watch_from_config && is_file_input);

    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    append_config_warning(
        &mut config_warning,
        validate_theme_syntax(&theme_selection, &ts),
    );
    set_theme_selection(theme_selection);
    let theme = current_syntect_theme(&ts).clone();
    runtime::debug_log(
        debug_input,
        &format!(
            "main input_ready filename={filename} filepath={} picker={} watch={}",
            filepath
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            picker,
            watch
        ),
    );

    let last_file_state = filepath.as_ref().and_then(read_file_state);
    let last_content_hash = hash_str(&src);

    let at = app_theme();
    if render {
        if let Some(warning) = config_warning.as_deref() {
            eprintln!("leaf: {warning}");
        }
        let mode = if ansi {
            ColorMode::Ansi
        } else if plain || !io::stdout().is_terminal() {
            ColorMode::Plain
        } else {
            ColorMode::Ansi
        };
        let width = terminal_render_width();
        let (lines, _) = parse_markdown_with_width(&src, &ss, &theme, width, &at.markdown);
        let mut stdout = io::stdout().lock();
        write_lines(&mut stdout, &lines, mode)?;
        return Ok(());
    }
    let (lines, toc) = parse_markdown(&src, &ss, &theme, &at.markdown);
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename,
            source: src,
            debug_input,
            watch,
            filepath,
            last_file_state,
        },
    );
    app.set_last_content_hash(last_content_hash);
    app.set_watch_from_config(watch_from_config);
    app.set_editor_config(Some(resolved_editor));
    app.set_config_warning(config_warning);
    if let Some(dir) = open_browser_picker_dir {
        app.queue_file_picker(dir);
    }
    if let Some(dir) = open_fuzzy_picker_dir {
        app.queue_fuzzy_file_picker(dir);
    }
    runtime::debug_log(
        debug_input,
        &format!(
            "main app_ready pending_picker={} picker_loading={}",
            app.has_pending_picker(),
            app.is_picker_loading()
        ),
    );

    let mut stdout = io::stdout();
    print!("\x1b]0;leaf\x07");
    let _ = io::stdout().flush();
    runtime::debug_log(debug_input, "terminal enter start");
    let mut session = TerminalSession::enter(&mut stdout)?;
    runtime::debug_log(debug_input, "terminal enter done");
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    runtime::debug_log(debug_input, "terminal new done");
    terminal.clear()?;
    runtime::debug_log(debug_input, "terminal clear done");
    let initial_draw_result = (|| -> Result<()> {
        let area = terminal.size()?;
        runtime::debug_log(
            debug_input,
            &format!(
                "initial_draw size width={} height={}",
                area.width, area.height
            ),
        );
        runtime::prepare_initial_picker_state(area.width as usize, &mut app, &ss, &ts)?;
        runtime::debug_log(debug_input, "initial_draw draw start");
        terminal.draw(|f| render::ui(f, &mut app))?;
        runtime::debug_log(debug_input, "initial_draw draw done");
        session.finish_initial_draw(&mut terminal)?;
        runtime::debug_log(debug_input, "initial_draw sync end done");
        Ok(())
    })();
    let run_result = match initial_draw_result {
        Ok(()) => {
            runtime::debug_log(debug_input, "run loop start");
            run(&mut terminal, &mut app, &ss, &ts, true)
        }
        Err(err) => Err(err),
    };
    runtime::debug_log(debug_input, "run loop end");
    let restore_result = session.restore(&mut terminal);
    finish_with_restore(run_result, restore_result)
}
