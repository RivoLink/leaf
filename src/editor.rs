use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const EDITOR_LINE_PLACEHOLDER: &str = "{$line}";
const EDITOR_PATH_PLACEHOLDER: &str = "{$path}";

const EDITOR_TAB_PREFIX: &str = "leaf editor: ";

fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(crate) fn expand_editor_placeholders(editor_cmd: &str, line: usize, path: &Path) -> String {
    editor_cmd
        .replace(EDITOR_LINE_PLACEHOLDER, &line.to_string())
        .replace(EDITOR_PATH_PLACEHOLDER, &path.display().to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorKind {
    Terminal,
    Gui,
}

pub(crate) fn binary_name(editor_cmd: &str) -> &str {
    let (bin, _) = split_editor_cmd(editor_cmd);
    let after_sep = bin
        .rsplit_once(['/', '\\'])
        .map(|(_, name)| name)
        .unwrap_or(bin);
    after_sep.strip_suffix(".exe").unwrap_or(after_sep)
}

pub(crate) fn classify(editor_cmd: &str) -> EditorKind {
    match binary_name(editor_cmd) {
        "code" | "codium" | "subl" | "gedit" | "kate" | "mousepad" | "notepad" | "notepad++"
        | "zed" | "xjed" | "termux-open" => EditorKind::Gui,
        _ => EditorKind::Terminal,
    }
}

fn tokenize_args(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    for c in s.chars() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

pub(crate) fn split_editor_cmd(cmd: &str) -> (&str, Vec<String>) {
    let trimmed = cmd.trim();

    for quote in ['"', '\''] {
        if trimmed.starts_with(quote) {
            if let Some(end) = trimmed[1..].find(quote) {
                let bin = &trimmed[1..=end];
                let rest = trimmed[end + 2..].trim();
                return (bin, tokenize_args(rest));
            }
        }
    }

    if !trimmed.contains(' ') && !trimmed.contains('\t') {
        return (trimmed, vec![]);
    }

    let first_space = trimmed.find([' ', '\t']).unwrap_or(trimmed.len());
    if trimmed[..first_space].contains('\\') {
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let mut split_at = parts.len();
        while split_at > 1 && parts[split_at - 1].starts_with('-') {
            split_at -= 1;
        }
        let bin_end = if split_at < parts.len() {
            parts[split_at].as_ptr() as usize - trimmed.as_ptr() as usize
        } else {
            trimmed.len()
        };
        let bin = trimmed[..bin_end].trim_end();
        let args: Vec<String> = parts[split_at..]
            .iter()
            .copied()
            .map(str::to_string)
            .collect();
        return (bin, args);
    }

    let bin = trimmed.split_whitespace().next().unwrap_or(trimmed);
    let rest = &trimmed[bin.len()..];
    (bin, tokenize_args(rest))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EditorEntry {
    pub(crate) name: String,
    pub(crate) command: String,
    pub(crate) kind: EditorKind,
}

const KNOWN_EDITORS: &[(&str, EditorKind)] = &[
    ("nano", EditorKind::Terminal),
    ("vim", EditorKind::Terminal),
    ("vi", EditorKind::Terminal),
    ("nvim", EditorKind::Terminal),
    ("micro", EditorKind::Terminal),
    ("hx", EditorKind::Terminal),
    ("emacs", EditorKind::Terminal),
    ("jed", EditorKind::Terminal),
    ("code", EditorKind::Gui),
    ("codium", EditorKind::Gui),
    ("subl", EditorKind::Gui),
    ("gedit", EditorKind::Gui),
    ("kate", EditorKind::Gui),
    ("mousepad", EditorKind::Gui),
    ("zed", EditorKind::Gui),
    ("xjed", EditorKind::Gui),
    ("notepad", EditorKind::Gui),
    ("notepad++", EditorKind::Gui),
];

pub(crate) fn which(bin: &str) -> Option<PathBuf> {
    if bin.contains('/') || bin.contains('\\') {
        let p = Path::new(bin);
        return p.is_file().then(|| p.to_path_buf());
    }
    let path_var = std::env::var("PATH").ok()?;
    let separator = if cfg!(target_os = "windows") {
        ';'
    } else {
        ':'
    };
    let candidates: Vec<String> = if cfg!(target_os = "windows") && !bin.contains('.') {
        vec![
            format!("{bin}.exe"),
            format!("{bin}.cmd"),
            format!("{bin}.bat"),
        ]
    } else {
        vec![bin.to_string()]
    };
    path_var.split(separator).find_map(|dir| {
        candidates
            .iter()
            .map(|name| Path::new(dir).join(name))
            .find(|p| p.is_file())
    })
}

pub(crate) fn scan_available_editors() -> Vec<EditorEntry> {
    let mut found = Vec::new();
    let is_termux = std::env::var("TERMUX_VERSION").is_ok();

    for &(name, kind) in KNOWN_EDITORS {
        if is_termux && kind == EditorKind::Gui {
            continue;
        }
        if which(name).is_some() {
            found.push(EditorEntry {
                name: name.to_string(),
                command: name.to_string(),
                kind,
            });
        }
    }

    if is_termux && which("termux-open").is_some() {
        found.push(EditorEntry {
            name: "intent".to_string(),
            command: "termux-open --chooser".to_string(),
            kind: EditorKind::Gui,
        });
    }

    found.sort_by_key(|e| match e.kind {
        EditorKind::Terminal => 0,
        EditorKind::Gui => 1,
    });
    found
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TerminalEmulator {
    Kitty,
    GnomeTerminal,
    MacTerminal(String),
    WindowsTerminal,
    Termux,
    Unknown,
}

pub(crate) fn detect_terminal_emulator() -> TerminalEmulator {
    if std::env::var("KITTY_PID").is_ok() {
        return TerminalEmulator::Kitty;
    }
    if std::env::var("GNOME_TERMINAL_SCREEN").is_ok() {
        return TerminalEmulator::GnomeTerminal;
    }
    if std::env::var("WT_SESSION").is_ok() {
        return TerminalEmulator::WindowsTerminal;
    }
    if std::env::var("TERMUX_VERSION").is_ok() {
        return TerminalEmulator::Termux;
    }

    if let Ok(tp) = std::env::var("TERM_PROGRAM") {
        match tp.as_str() {
            "iTerm.app" | "iTerm2" | "Apple_Terminal" => {
                return TerminalEmulator::MacTerminal(tp);
            }
            _ => {}
        }
    }

    TerminalEmulator::Unknown
}

const SELECT_HINT_SHIFT: &str = "shift+drag";
const SELECT_HINT_OPTION: &str = "option+drag";

pub(crate) fn selection_modifier_label(emulator: &TerminalEmulator) -> &'static str {
    match emulator {
        TerminalEmulator::MacTerminal(tp) if tp == "iTerm.app" || tp == "iTerm2" => {
            SELECT_HINT_OPTION
        }
        _ => SELECT_HINT_SHIFT,
    }
}

pub(crate) fn resolve_editor(cli_editor: Option<&str>, config_editor: Option<&str>) -> String {
    let raw = if let Some(e) = cli_editor {
        e.to_string()
    } else if let Some(e) = std::env::var("LEAF_EDITOR").ok().filter(|s| !s.is_empty()) {
        e
    } else if let Some(e) = config_editor {
        e.to_string()
    } else {
        platform_fallback_editor().to_string()
    };
    expand_editor_alias(&raw)
}

fn expand_editor_alias(editor: &str) -> String {
    match editor.trim() {
        "intent" => "termux-open --chooser".to_string(),
        _ => editor.to_string(),
    }
}

pub(crate) fn resolve_fallback_editor(editor_cmd: &str) -> Option<&'static str> {
    let fallback = platform_fallback_editor();
    if binary_name(editor_cmd) != binary_name(fallback) {
        Some(fallback)
    } else {
        None
    }
}

fn platform_fallback_editor() -> &'static str {
    if cfg!(target_os = "windows") {
        "notepad"
    } else {
        "nano"
    }
}

fn is_zellij() -> bool {
    std::env::var_os("ZELLIJ").is_some()
}

fn is_wsl() -> bool {
    static CACHED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHED.get_or_init(|| {
        if std::env::var("WSL_DISTRO_NAME")
            .map(|s| !s.is_empty())
            .unwrap_or(false)
        {
            return true;
        }
        std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|s| s.to_lowercase().contains("microsoft"))
            .unwrap_or(false)
    })
}

pub(crate) enum LaunchStrategy {
    SpawnAndAssume(Command),
    RunAndCheck(Command),
}

fn silence(cmd: &mut Command) {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
}

impl LaunchStrategy {
    fn run(self) -> bool {
        match self {
            LaunchStrategy::SpawnAndAssume(mut cmd) => {
                silence(&mut cmd);
                cmd.spawn().is_ok()
            }
            LaunchStrategy::RunAndCheck(mut cmd) => {
                silence(&mut cmd);
                cmd.status().map(|s| s.success()).unwrap_or(false)
            }
        }
    }
}

pub(crate) fn format_editor_tab_title(file: &Path, tab_title_length: Option<i32>) -> String {
    let name = file
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.display().to_string());
    let max_filename_len = tab_title_length
        .and_then(|n| crate::max_filename_len_for_prefix(n, EDITOR_TAB_PREFIX.len()));
    let display = match max_filename_len {
        Some(m) => crate::terminal::format_tab_title_filename(&name, m),
        None => name,
    };
    format!("{EDITOR_TAB_PREFIX}{display}")
}

// Inside zellij the host emulator is the wrong target: its new tab would land
// outside the session. Open a zellij pane in place of leaf's own pane instead.
// The editor is zellij's (`scrollback_editor`, else $EDITOR), so leaf's editor
// command is not used here -- only the file and the visible line.
fn zellij_edit_command(file: &Path, line: usize) -> LaunchStrategy {
    let mut cmd = Command::new("zellij");
    cmd.arg("action")
        .arg("edit")
        .arg("--in-place")
        .arg("--line-number")
        .arg(line.max(1).to_string())
        .arg(file);
    LaunchStrategy::RunAndCheck(cmd)
}

pub(crate) fn try_new_tab_command(
    editor: &str,
    file: &Path,
    line: usize,
    emulator: &TerminalEmulator,
    wsl: bool,
    zellij: bool,
    tab_title_length: Option<i32>,
) -> Option<LaunchStrategy> {
    if zellij {
        return Some(zellij_edit_command(file, line));
    }

    let (bin, args) = split_editor_cmd(editor);
    let file_str = file.display().to_string();
    let title = format_editor_tab_title(file, tab_title_length);

    match emulator {
        TerminalEmulator::Kitty => {
            let mut cmd = Command::new("kitty");
            cmd.arg("@")
                .arg("launch")
                .arg("--type=tab")
                .arg(format!("--tab-title={title}"))
                .arg(bin);
            for a in &args {
                cmd.arg(a);
            }
            cmd.arg(&file_str);
            Some(LaunchStrategy::RunAndCheck(cmd))
        }
        TerminalEmulator::GnomeTerminal => {
            let mut cmd = Command::new("gnome-terminal");
            cmd.arg("--tab")
                .arg(format!("--title={title}"))
                .arg("--")
                .arg(bin);
            for a in &args {
                cmd.arg(a);
            }
            cmd.arg(&file_str);
            Some(LaunchStrategy::SpawnAndAssume(cmd))
        }
        TerminalEmulator::MacTerminal(ref tp) => {
            let app_name = if tp == "Apple_Terminal" {
                "Terminal"
            } else {
                "iTerm"
            };
            let escaped_editor = escape_applescript_string(editor);
            let escaped_file = escape_applescript_string(&file_str);
            let script = if tp == "Apple_Terminal" {
                let shell_title = title.replace('\'', "'\\''");
                let escaped_title = escape_applescript_string(&shell_title);
                let title_seq = format!(r#"printf '\\033]0;{escaped_title}\\007'; "#);
                format!(
                    "tell application \"{app_name}\" to do script \"{title_seq}{escaped_editor} {escaped_file}\""
                )
            } else {
                format!(
                    "tell application \"{app_name}\" to tell current window to \
                     create tab with default profile command \"{escaped_editor} {escaped_file}\""
                )
            };
            let mut cmd = Command::new("osascript");
            cmd.arg("-e").arg(script);
            Some(LaunchStrategy::SpawnAndAssume(cmd))
        }
        TerminalEmulator::WindowsTerminal => {
            let is_windows_bin = Path::new(bin)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("exe"));
            if wsl && !is_windows_bin {
                return None;
            }
            let mut cmd = Command::new("wt");
            cmd.arg("new-tab").arg("--title").arg(&title).arg(bin);
            for a in &args {
                cmd.arg(a);
            }
            cmd.arg(&file_str);
            Some(LaunchStrategy::SpawnAndAssume(cmd))
        }
        TerminalEmulator::Termux | TerminalEmulator::Unknown => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorResult {
    Opened,
    NeedsSameTerminal,
}

pub(crate) fn open_in_editor(
    editor: &str,
    file: &Path,
    line: usize,
    kind: EditorKind,
    emulator: &TerminalEmulator,
    tab_title_length: Option<i32>,
) -> Result<EditorResult, String> {
    let (bin, args) = split_editor_cmd(editor);
    match kind {
        EditorKind::Gui => {
            let exec = which(bin).unwrap_or_else(|| PathBuf::from(bin));
            let mut cmd = Command::new(&exec);
            cmd.args(&args).arg(file);
            silence(&mut cmd);
            cmd.spawn().map_err(|e| format!("{bin}: {e}"))?;
            Ok(EditorResult::Opened)
        }
        EditorKind::Terminal => {
            if try_new_tab_command(
                editor,
                file,
                line,
                emulator,
                is_wsl(),
                is_zellij(),
                tab_title_length,
            )
            .map(LaunchStrategy::run)
            .unwrap_or(false)
            {
                return Ok(EditorResult::Opened);
            }
            Ok(EditorResult::NeedsSameTerminal)
        }
    }
}
