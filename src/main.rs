use anyhow::{bail, Context, Result};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{fs::OpenOptions, io, io::IsTerminal, io::Read, io::Write, path::PathBuf};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

mod app;
mod cli;
mod clipboard;
mod completions;
mod config;
mod editor;
mod inline;
mod markdown;
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
pub(crate) use editor::{
    binary_name, classify, expand_editor_placeholders, resolve_editor, selection_modifier_label,
    split_editor_cmd, try_new_tab_command, EditorKind, TerminalEmulator,
};
#[cfg(test)]
pub(crate) use markdown::toc::{normalize_toc, toc_levels, TocEntry};
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

fn resolve_configured_width(
    cli_width: Option<usize>,
    config_width: Option<usize>,
) -> Option<usize> {
    if let Some(w) = cli_width {
        return Some(w);
    }
    if let Ok(val) = std::env::var("LEAF_WIDTH") {
        if let Ok(w) = val.parse::<usize>() {
            if w >= 20 {
                return Some(w);
            }
        }
    }
    config_width.map(|w| w.max(20))
}

fn resolve_code_line_numbers(config_value: Option<bool>) -> bool {
    if let Ok(val) = std::env::var("LEAF_CODE_LINE_NUMBERS") {
        match val.as_str() {
            "1" => return true,
            "0" => return false,
            _ => {}
        }
    }
    config_value.unwrap_or(true)
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
    if let Some(ref config_action) = options.config {
        match config_action {
            cli::ConfigAction::Open => config::run_config()?,
            cli::ConfigAction::Reset => config::reset_config()?,
        }
        return Ok(());
    }
    if let Some(ref ac_arg) = options.auto_complete {
        completions::run_auto_complete(ac_arg)?;
        return Ok(());
    }
    let CliOptions {
        picker,
        watch: watch_from_cli,
        debug_input,
        file_arg,
        theme: cli_theme,
        editor: cli_editor,
        inline: mut inline_spec,
        width: cli_width,
        ..
    } = options;

    let overrides = config::CliOverrides {
        width: cli_width,
        theme: cli_theme.clone(),
    };
    let (user_config, mut config_warning) = config::load_config(&overrides);

    let theme_selection = if let Some(theme_name) = cli_theme.as_deref() {
        resolve_theme_selection(theme_name, &user_config.themes, None)
            .map_err(|message| anyhow::anyhow!("{message}"))?
    } else if let Some(theme_name) = std::env::var("LEAF_THEME")
        .ok()
        .filter(|s| !s.is_empty())
        .as_deref()
    {
        resolve_theme_selection(theme_name, &user_config.themes, None).unwrap_or_default()
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
    let max_width = resolve_configured_width(cli_width, user_config.width);
    let code_line_numbers = resolve_code_line_numbers(user_config.code_line_numbers);

    if let Some(ref mut spec) = inline_spec {
        if spec.width.is_none() {
            spec.width = max_width;
        }
    }

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
    let mut dir_arg = None;
    let (src, filename, filepath) = if let Some(f) = file_arg {
        let path = PathBuf::from(&f);
        if path.is_dir() {
            let label = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());
            if picker {
                open_browser_picker_dir = Some(path.clone());
            } else {
                open_fuzzy_picker_dir = Some(path.clone());
            }
            dir_arg = Some(path);
            (String::new(), label, None)
        } else if picker {
            anyhow::bail!("--picker cannot be combined with a file path");
        } else {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Cannot read: {}", path.display()))?;
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or(f);
            (content, name, Some(path))
        }
    } else {
        if io::stdin().is_terminal() {
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

    let ext = filepath
        .as_ref()
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let (src, file_mode) = App::wrap_as_code_block(src, ext, &ss);

    if let Some(ref spec) = inline_spec {
        if src.is_empty() && filepath.is_none() {
            bail!("--inline requires a file path or stdin input");
        }

        let is_tty = io::stdout().is_terminal();
        let width = inline::render_width(spec, is_tty);
        let format = inline::resolve_format(spec, is_tty);

        let at = app_theme();
        let mut parsed = parse_markdown_with_width(
            &src,
            &ss,
            &theme,
            width,
            &at.markdown,
            file_mode,
            code_line_numbers,
        );

        while parsed.lines.last().is_some_and(|l| {
            l.spans.is_empty() || l.spans.iter().all(|s| s.content.trim().is_empty())
        }) {
            parsed.lines.pop();
        }
        let lines = parsed.lines;

        let stdout = io::stdout();
        let mut writer = io::BufWriter::new(stdout.lock());
        inline::write_lines(&lines, format, width, &mut writer)?;
        return Ok(());
    }

    let at = app_theme();
    let parsed = parse_markdown(
        &src,
        &ss,
        &theme,
        &at.markdown,
        file_mode,
        code_line_numbers,
    );
    let crate::markdown::ParseResult {
        lines,
        toc,
        link_spans,
        line_number_map,
        source_line_map,
        code_blocks,
    } = parsed;
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
    app.set_link_spans(link_spans);
    app.set_code_blocks(code_blocks);
    app.set_line_maps(line_number_map, source_line_map);
    app.set_last_content_hash(last_content_hash);
    app.set_watch_from_config(watch_from_config);
    app.set_max_width(max_width);
    app.set_extras(user_config.extras);
    app.set_file_mode(file_mode);
    app.set_editor_config(Some(resolved_editor));
    app.set_code_line_numbers(code_line_numbers);
    app.set_config_warning(config_warning);
    if let Some(dir) = dir_arg {
        app.set_dir_arg(dir);
    }
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
