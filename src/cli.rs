use anyhow::{bail, Result};

use crate::inline::{self, InlineSpec};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConfigAction {
    Open,
    Reset,
    Remove,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HistoryAction {
    Picker,
    Edit,
    Remove,
    List { count: Option<usize> },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AutoCompleteMode {
    Install,
    Dump,
    Remove,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AutoCompleteArg {
    pub(crate) shell: Option<String>,
    pub(crate) mode: AutoCompleteMode,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CliOptions {
    pub(crate) picker: bool,
    pub(crate) watch: bool,
    pub(crate) update: bool,
    pub(crate) config: Option<ConfigAction>,
    pub(crate) auto_complete: Option<AutoCompleteArg>,
    pub(crate) debug_input: bool,
    pub(crate) print_help: bool,
    pub(crate) print_version: bool,
    pub(crate) file_arg: Option<String>,
    pub(crate) theme: Option<String>,
    pub(crate) editor: Option<String>,
    pub(crate) inline: Option<InlineSpec>,
    pub(crate) width: Option<usize>,
    pub(crate) history: Option<HistoryAction>,
    pub(crate) fuzzy: bool,
    pub(crate) fuzzy_query: Option<String>,
}

pub(crate) const FUZZY_QUERY_MAX_LEN: usize = 15;
const FUZZY_QUERY_CHARSET_DESC: &str = "[A-Za-z0-9._-]";

pub(crate) fn is_valid_fuzzy_query(s: &str) -> bool {
    !s.is_empty()
        && s.chars().count() <= FUZZY_QUERY_MAX_LEN
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn parse_fuzzy_query(value: String) -> Result<String> {
    if !is_valid_fuzzy_query(&value) {
        anyhow::bail!(
            "--fuzzy: invalid value '{value}'\n\
             \x20      allowed: {FUZZY_QUERY_CHARSET_DESC}, max {FUZZY_QUERY_MAX_LEN} chars"
        );
    }
    Ok(value)
}

pub(crate) fn usage_text() -> &'static str {
    "Usage:  leaf [OPTIONS] [file.md|directory|keyword]\n\
     \x20       leaf [--watch] --picker\n\
     \x20       leaf --update\n\
     \x20       echo '# Hello' | leaf\n\
     \n\
     Options:\n\
     \x20 -h, --help                   Show this help message and exit\n\
     \x20 -V, --version                Show version information and exit\n\
     \x20 -w, --watch                  Watch the file for changes and reload automatically\n\
     \x20     --theme <NAME>           Set color theme preset or custom config theme\n\
     \x20 -e, --editor <NAME>          Set external editor (nano|vim|code|subl|emacs)\n\
     \x20     --inline [SPEC]          Render to stdout (no TUI) [ansi|plain][:<width>]\n\
     \x20     --width <N>              Set maximum content width (min: 20)\n\
     \x20     --fuzzy [KEYWORD]        Open the fuzzy file picker (KEYWORD pre-fills the filter)\n\
     \x20     --picker                 Open the file browser picker\n\
     \x20 -H, --history [SPEC]         Open picker, or [edit|remove|list:<n>] file history\n\
     \x20     --config [reset|remove]  Open, reset or remove configuration\n\
     \x20     --update                 Update leaf to the latest version\n\
     \x20     --auto-complete [SPEC]   Install, dump or remove shell completions [<shell>][:dump|:remove]"
}

pub(crate) fn version_text() -> &'static str {
    concat!("leaf ", env!("CARGO_PKG_VERSION"))
}

pub(crate) fn print_usage() {
    println!("{}", usage_text());
}

pub(crate) fn print_version() {
    println!("{}", version_text());
}

pub(crate) fn parse_cli(args: &[String]) -> Result<CliOptions> {
    let mut options = CliOptions::default();
    let mut positional_only = false;
    let mut iter = args.iter().skip(1).peekable();

    while let Some(arg) = iter.next() {
        if positional_only {
            if options.file_arg.is_none() {
                options.file_arg = Some(arg.clone());
            } else {
                anyhow::bail!("Too many file arguments");
            }
            continue;
        }

        match arg.as_str() {
            "--picker" => options.picker = true,
            "--fuzzy" => {
                options.fuzzy = true;
                let take_value = iter
                    .peek()
                    .map(|next| !next.starts_with('-'))
                    .unwrap_or(false);
                if take_value {
                    options.fuzzy_query = Some(parse_fuzzy_query(iter.next().unwrap().clone())?);
                }
            }
            _ if arg.starts_with("--fuzzy=") => {
                options.fuzzy = true;
                options.fuzzy_query = Some(parse_fuzzy_query(arg["--fuzzy=".len()..].to_string())?);
            }
            "--watch" | "-w" => options.watch = true,
            "--update" => options.update = true,
            "--config" => {
                let action = match iter.peek().map(|s| s.as_str()) {
                    Some("reset") => {
                        iter.next();
                        ConfigAction::Reset
                    }
                    Some("remove") => {
                        iter.next();
                        ConfigAction::Remove
                    }
                    _ => ConfigAction::Open,
                };
                options.config = Some(action);
            }
            "--auto-complete" => {
                let ac_arg = match iter.peek() {
                    Some(next) if !next.starts_with('-') => {
                        let value = iter.next().unwrap();
                        parse_auto_complete_value(value)?
                    }
                    _ => AutoCompleteArg {
                        shell: None,
                        mode: AutoCompleteMode::Install,
                    },
                };
                options.auto_complete = Some(ac_arg);
            }
            "--debug-input" => options.debug_input = true,
            "--help" | "-h" => options.print_help = true,
            "--version" | "-V" => options.print_version = true,
            "--history" | "-H" => {
                let action = match iter.peek().map(|s| s.as_str()) {
                    Some("edit") => {
                        iter.next();
                        HistoryAction::Edit
                    }
                    Some("remove") => {
                        iter.next();
                        HistoryAction::Remove
                    }
                    Some(next) if !next.starts_with('-') => {
                        let value = iter.next().unwrap();
                        parse_history_list_spec(value)?
                    }
                    _ => HistoryAction::Picker,
                };
                options.history = Some(action);
            }
            "--theme" => {
                let Some(name) = iter.next() else {
                    anyhow::bail!("Missing value for --theme");
                };
                options.theme = Some(parse_theme_name(name)?);
            }
            _ if arg.starts_with("--theme=") => {
                let name = &arg["--theme=".len()..];
                options.theme = Some(parse_theme_name(name)?);
            }
            "--editor" | "-e" => {
                let Some(value) = iter.next() else {
                    anyhow::bail!("Missing value for --editor");
                };
                options.editor = Some(value.clone());
            }
            _ if arg.starts_with("--editor=") => {
                options.editor = Some(arg["--editor=".len()..].to_string());
            }
            "--inline" => {
                let spec = match iter.peek() {
                    Some(next) if inline::is_inline_spec(next) => {
                        let value = iter.next().unwrap();
                        inline::parse_inline_spec(value)?
                    }
                    _ => InlineSpec {
                        format: inline::InlineFormat::Auto,
                        width: None,
                    },
                };
                options.inline = Some(spec);
            }
            _ if arg.starts_with("--inline=") => {
                let value = &arg["--inline=".len()..];
                options.inline = Some(inline::parse_inline_spec(value)?);
            }
            "--width" => {
                let Some(value) = iter.next() else {
                    anyhow::bail!("Missing value for --width");
                };
                options.width = Some(parse_width_value(value)?);
            }
            _ if arg.starts_with("--width=") => {
                let value = &arg["--width=".len()..];
                options.width = Some(parse_width_value(value)?);
            }
            "--" => positional_only = true,
            _ if arg.starts_with('-') => anyhow::bail!("Unknown flag: {arg}"),
            _ if options.file_arg.is_none() => options.file_arg = Some(arg.clone()),
            _ => anyhow::bail!("Too many file arguments"),
        }
    }

    let standalone = [
        (options.update, "--update"),
        (options.config.is_some(), "--config"),
        (options.auto_complete.is_some(), "--auto-complete"),
        (options.history.is_some(), "--history"),
    ];
    let standalone_count = standalone.iter().filter(|(set, _)| *set).count();
    for &(set, name) in &standalone {
        if !set {
            continue;
        }
        let has_other = standalone_count > 1
            || options.watch
            || options.picker
            || options.fuzzy
            || options.debug_input
            || options.file_arg.is_some()
            || options.theme.is_some()
            || options.editor.is_some();
        if has_other {
            anyhow::bail!("{name} must be used on its own");
        }
    }

    if options.inline.is_some() {
        if options.watch {
            anyhow::bail!("--inline cannot be combined with --watch");
        }
        if options.picker {
            anyhow::bail!("--inline cannot be combined with --picker");
        }
        if options.fuzzy {
            anyhow::bail!("--inline cannot be combined with --fuzzy");
        }
    }

    if options.fuzzy {
        if options.picker {
            anyhow::bail!("--fuzzy cannot be combined with --picker");
        }
        if options.file_arg.is_some() {
            anyhow::bail!("--fuzzy cannot be combined with a file argument");
        }
    }

    Ok(options)
}

fn parse_theme_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("Missing value for --theme");
    }
    Ok(name.to_string())
}

const KNOWN_SHELLS: &[&str] = &["bash", "zsh", "fish", "powershell"];

fn parse_auto_complete_value(s: &str) -> Result<AutoCompleteArg> {
    let (shell_part, mode) = match s.split_once(':') {
        Some((shell, "dump")) => (Some(shell), AutoCompleteMode::Dump),
        Some((shell, "remove")) => (Some(shell), AutoCompleteMode::Remove),
        Some(_) => bail!(
            "Invalid argument for --auto-complete: '{s}'. \
             Expected: bash, zsh, fish, powershell, dump, remove, SHELL:dump, or SHELL:remove"
        ),
        None => match s {
            "dump" => (None, AutoCompleteMode::Dump),
            "remove" => (None, AutoCompleteMode::Remove),
            _ => (Some(s), AutoCompleteMode::Install),
        },
    };
    let shell = match shell_part {
        Some(name) if KNOWN_SHELLS.contains(&name) => Some(name.to_string()),
        Some(name) => bail!("Unknown shell: '{name}'. Expected: bash, zsh, fish, powershell"),
        None => None,
    };
    Ok(AutoCompleteArg { shell, mode })
}

fn parse_history_list_spec(s: &str) -> Result<HistoryAction> {
    if s == "list" {
        return Ok(HistoryAction::List { count: None });
    }
    let count_str = s.strip_prefix("list:").unwrap_or(s);
    if let Some(n) = count_str.parse::<usize>().ok().filter(|&n| n > 0) {
        return Ok(HistoryAction::List { count: Some(n) });
    }
    bail!(
        "--history: invalid value '{s}'\n\
         \x20       expected: edit, remove, list[:<n>], with n positive"
    )
}

fn parse_width_value(s: &str) -> Result<usize> {
    let w: usize = s
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid width: {s}"))?;
    if w < 20 {
        bail!("Width must be at least 20");
    }
    Ok(w)
}
