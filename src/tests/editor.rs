use crate::*;
use std::path::Path;

#[test]
fn binary_name_simple() {
    assert_eq!(binary_name("nano"), "nano");
}

#[test]
fn binary_name_full_path() {
    assert_eq!(binary_name("/usr/bin/code"), "code");
}

#[test]
fn binary_name_with_args() {
    assert_eq!(binary_name("emacs -nw"), "emacs");
}

#[test]
fn binary_name_path_with_args() {
    assert_eq!(binary_name("/usr/bin/emacs -nw"), "emacs");
}

#[test]
fn binary_name_windows() {
    assert_eq!(binary_name("notepad.exe"), "notepad");
}

#[test]
fn classify_gui_editors() {
    assert_eq!(classify("code"), EditorKind::Gui);
    assert_eq!(classify("codium"), EditorKind::Gui);
    assert_eq!(classify("subl"), EditorKind::Gui);
    assert_eq!(classify("gedit"), EditorKind::Gui);
    assert_eq!(classify("kate"), EditorKind::Gui);
    assert_eq!(classify("mousepad"), EditorKind::Gui);
    assert_eq!(classify("notepad.exe"), EditorKind::Gui);
    assert_eq!(classify("notepad++"), EditorKind::Gui);
    assert_eq!(classify("zed"), EditorKind::Gui);
    assert_eq!(classify("xjed"), EditorKind::Gui);
}

#[test]
fn classify_terminal_editors() {
    assert_eq!(classify("nano"), EditorKind::Terminal);
    assert_eq!(classify("vim"), EditorKind::Terminal);
    assert_eq!(classify("nvim"), EditorKind::Terminal);
    assert_eq!(classify("micro"), EditorKind::Terminal);
    assert_eq!(classify("hx"), EditorKind::Terminal);
    assert_eq!(classify("emacs"), EditorKind::Terminal);
    assert_eq!(classify("jed"), EditorKind::Terminal);
}

#[test]
fn classify_unknown_defaults_to_terminal() {
    assert_eq!(classify("some-unknown-editor"), EditorKind::Terminal);
}

#[test]
fn classify_full_path() {
    assert_eq!(classify("/usr/bin/code"), EditorKind::Gui);
    assert_eq!(classify("/usr/local/bin/nano"), EditorKind::Terminal);
}

#[test]
fn classify_with_args() {
    assert_eq!(classify("emacs -nw"), EditorKind::Terminal);
    assert_eq!(classify("/usr/bin/code --new-window"), EditorKind::Gui);
}

#[test]
fn split_editor_cmd_simple() {
    let (bin, args) = split_editor_cmd("nano");
    assert_eq!(bin, "nano");
    assert!(args.is_empty());
}

#[test]
fn split_editor_cmd_with_args() {
    let (bin, args) = split_editor_cmd("emacs -nw");
    assert_eq!(bin, "emacs");
    assert_eq!(args, vec!["-nw"]);
}

#[test]
fn split_editor_cmd_path_with_args() {
    let (bin, args) = split_editor_cmd("/usr/bin/emacs -nw --no-splash");
    assert_eq!(bin, "/usr/bin/emacs");
    assert_eq!(args, vec!["-nw", "--no-splash"]);
}

#[test]
fn split_editor_cmd_inner_double_quotes() {
    let (bin, args) = split_editor_cmd(r#""C:\Program Files\Notepad++\notepad++.exe" --arg"#);
    assert_eq!(bin, r"C:\Program Files\Notepad++\notepad++.exe");
    assert_eq!(args, vec!["--arg"]);
}

#[test]
fn split_editor_cmd_inner_double_quotes_no_args() {
    let (bin, args) = split_editor_cmd(r#""C:\Program Files\Notepad++\notepad++.exe""#);
    assert_eq!(bin, r"C:\Program Files\Notepad++\notepad++.exe");
    assert!(args.is_empty());
}

#[test]
fn split_editor_cmd_inner_single_quotes() {
    let (bin, args) = split_editor_cmd("'/opt/My Apps/editor' -nw");
    assert_eq!(bin, "/opt/My Apps/editor");
    assert_eq!(args, vec!["-nw"]);
}

#[test]
fn split_editor_cmd_windows_path_no_args() {
    let (bin, args) = split_editor_cmd(r"C:\Program Files\Notepad++\notepad++.exe");
    assert_eq!(bin, r"C:\Program Files\Notepad++\notepad++.exe");
    assert!(args.is_empty());
}

#[test]
fn split_editor_cmd_windows_path_trailing_args() {
    let (bin, args) = split_editor_cmd(r"C:\Program Files\Notepad++\notepad++.exe --no-session");
    assert_eq!(bin, r"C:\Program Files\Notepad++\notepad++.exe");
    assert_eq!(args, vec!["--no-session"]);
}

#[test]
fn split_editor_cmd_windows_path_duplicate_trailing_args() {
    let (bin, args) = split_editor_cmd(r"C:\Program Files\app.exe -nw -nw");
    assert_eq!(bin, r"C:\Program Files\app.exe");
    assert_eq!(args, vec!["-nw", "-nw"]);
}

#[test]
fn split_editor_cmd_unix_path_with_args() {
    let (bin, args) = split_editor_cmd("/usr/bin/emacs -nw --no-splash");
    assert_eq!(bin, "/usr/bin/emacs");
    assert_eq!(args, vec!["-nw", "--no-splash"]);
}

#[test]
fn binary_name_windows_path_with_spaces() {
    assert_eq!(
        binary_name(r"C:\Program Files\Notepad++\notepad++.exe"),
        "notepad++"
    );
}

#[test]
fn binary_name_quoted_windows_path() {
    assert_eq!(
        binary_name(r#""C:\Program Files\Notepad++\notepad++.exe" --arg"#),
        "notepad++"
    );
}

#[test]
fn classify_windows_path_with_spaces() {
    assert_eq!(
        classify(r"C:\Program Files\Notepad++\notepad++.exe"),
        EditorKind::Gui
    );
}

fn mac_tab_script(editor: &str, file: &str, term_program: &str) -> String {
    let emulator = TerminalEmulator::MacTerminal(term_program.to_string());
    let strategy = try_new_tab_command(editor, Path::new(file), &emulator, false).unwrap();
    let cmd = match strategy {
        LaunchStrategy::SpawnAndAssume(cmd) => cmd,
        LaunchStrategy::RunAndCheck(_) => panic!("expected SpawnAndAssume for MacTerminal"),
    };
    let args: Vec<_> = cmd.get_args().collect();
    args[1].to_str().unwrap().to_string()
}

#[test]
fn try_new_tab_command_windows_terminal_wsl_linux_editor_returns_none() {
    let emulator = TerminalEmulator::WindowsTerminal;
    let strategy = try_new_tab_command("nano", Path::new("/tmp/test.md"), &emulator, true);
    assert!(strategy.is_none());
}

#[test]
fn try_new_tab_command_windows_terminal_wsl_windows_editor_returns_spawn() {
    let emulator = TerminalEmulator::WindowsTerminal;
    let strategy =
        try_new_tab_command("notepad.exe", Path::new("/tmp/test.md"), &emulator, true).unwrap();
    assert!(matches!(strategy, LaunchStrategy::SpawnAndAssume(_)));
}

#[test]
fn try_new_tab_command_windows_terminal_non_wsl_returns_spawn() {
    let emulator = TerminalEmulator::WindowsTerminal;
    let strategy =
        try_new_tab_command("nano", Path::new("/tmp/test.md"), &emulator, false).unwrap();
    assert!(matches!(strategy, LaunchStrategy::SpawnAndAssume(_)));
}

#[test]
fn try_new_tab_command_kitty_returns_run_and_check() {
    let emulator = TerminalEmulator::Kitty;
    let strategy =
        try_new_tab_command("nano", Path::new("/tmp/test.md"), &emulator, false).unwrap();
    assert!(matches!(strategy, LaunchStrategy::RunAndCheck(_)));
}

#[test]
fn try_new_tab_command_gnome_returns_spawn() {
    let emulator = TerminalEmulator::GnomeTerminal;
    let strategy = try_new_tab_command("vim", Path::new("/tmp/test.md"), &emulator, false).unwrap();
    assert!(matches!(strategy, LaunchStrategy::SpawnAndAssume(_)));
}

#[test]
fn try_new_tab_command_termux_returns_none() {
    let emulator = TerminalEmulator::Termux;
    let strategy = try_new_tab_command("nano", Path::new("/tmp/test.md"), &emulator, false);
    assert!(strategy.is_none());
}

#[test]
fn try_new_tab_command_unknown_returns_none() {
    let emulator = TerminalEmulator::Unknown;
    let strategy = try_new_tab_command("nano", Path::new("/tmp/test.md"), &emulator, false);
    assert!(strategy.is_none());
}

#[test]
fn new_tab_command_apple_terminal_has_printf() {
    let script = mac_tab_script("nano", "/tmp/test.md", "Apple_Terminal");
    assert!(script.contains("printf"));
    assert!(script.contains("do script"));
    assert!(script.contains("nano"));
    assert!(script.contains("/tmp/test.md"));
}

#[test]
fn new_tab_command_iterm_no_printf() {
    let script = mac_tab_script("nano", "/tmp/test.md", "iTerm.app");
    assert!(!script.contains("printf"));
    assert!(script.contains("create tab with default profile command"));
    assert!(script.contains("nano"));
    assert!(script.contains("/tmp/test.md"));
}

#[test]
fn selection_modifier_label_iterm_is_option() {
    assert_eq!(
        selection_modifier_label(&TerminalEmulator::MacTerminal("iTerm.app".into())),
        "option+drag"
    );
    assert_eq!(
        selection_modifier_label(&TerminalEmulator::MacTerminal("iTerm2".into())),
        "option+drag"
    );
}

#[test]
fn selection_modifier_label_apple_terminal_is_shift() {
    assert_eq!(
        selection_modifier_label(&TerminalEmulator::MacTerminal("Apple_Terminal".into())),
        "shift+drag"
    );
}

#[test]
fn selection_modifier_label_other_terminals_are_shift() {
    for term in [
        TerminalEmulator::Kitty,
        TerminalEmulator::GnomeTerminal,
        TerminalEmulator::WindowsTerminal,
        TerminalEmulator::Termux,
        TerminalEmulator::Unknown,
    ] {
        assert_eq!(selection_modifier_label(&term), "shift+drag");
    }
}

#[test]
fn new_tab_command_iterm2_no_printf() {
    let script = mac_tab_script("vim", "/tmp/test.md", "iTerm2");
    assert!(!script.contains("printf"));
    assert!(script.contains("create tab with default profile command"));
    assert!(script.contains("vim"));
}

#[test]
fn new_tab_command_iterm_file_with_spaces() {
    let script = mac_tab_script("nano", "/tmp/my file.md", "iTerm.app");
    assert!(!script.contains("printf"));
    assert!(script.contains("my file.md"));
}

#[test]
fn new_tab_command_apple_terminal_file_with_spaces() {
    let script = mac_tab_script("nano", "/tmp/my file.md", "Apple_Terminal");
    assert!(script.contains("printf"));
    assert!(script.contains("my file.md"));
}

#[test]
fn resolve_editor_cli_takes_priority() {
    let result = resolve_editor(Some("vim"), None);
    assert_eq!(result, "vim");
}

#[test]
fn resolve_editor_fallback_is_not_empty() {
    let result = resolve_editor(None, None);
    assert!(!result.is_empty());
}

#[test]
fn resolve_editor_config_takes_priority_over_fallback() {
    let result = resolve_editor(None, Some("hx"));
    assert_eq!(result, "hx");
}

#[test]
fn expand_editor_placeholders_no_placeholder_returns_unchanged() {
    let result = expand_editor_placeholders("nvim", 42, Path::new(""));
    assert_eq!(result, "nvim");
}

#[test]
fn expand_editor_placeholders_substitutes_single_occurrence() {
    let result = expand_editor_placeholders("nvim +{$line}", 42, Path::new(""));
    assert_eq!(result, "nvim +42");
}

#[test]
fn expand_editor_placeholders_substitutes_all_occurrences() {
    let result = expand_editor_placeholders("code -g {$line}:{$line}", 7, Path::new(""));
    assert_eq!(result, "code -g 7:7");
}

#[test]
fn expand_editor_placeholders_preserves_surrounding_chars() {
    let result = expand_editor_placeholders(r#"nvim +{$line} +"normal! zz""#, 123, Path::new(""));
    assert_eq!(result, r#"nvim +123 +"normal! zz""#);
}

#[test]
fn expand_editor_placeholders_ignores_unsupported_variants() {
    let result = expand_editor_placeholders("nvim +${line} +{line} +{$LINE}", 5, Path::new(""));
    assert_eq!(result, "nvim +${line} +{line} +{$LINE}");
}

#[test]
fn expand_editor_placeholders_line_and_path() {
    let result = expand_editor_placeholders("code -g {$path}:{$line}", 0, Path::new("file.rs"));
    assert_eq!(result, "code -g file.rs:0");
}

#[test]
fn expand_editor_placeholders_multiple_occurrences() {
    let result =
        expand_editor_placeholders("code {$path} && echo {$path}", 1, Path::new("test.md"));
    assert_eq!(result, "code test.md && echo test.md");
}

#[test]
fn expand_editor_placeholders_full_path() {
    let result =
        expand_editor_placeholders("code -g {$path}:{$line}", 8, Path::new("/tmp/file.rs"));
    assert_eq!(result, "code -g /tmp/file.rs:8");
}

#[test]
fn split_editor_cmd_quoted_arg_with_space() {
    let (bin, args) = split_editor_cmd(r#"nvim +123 +"normal! zz""#);
    assert_eq!(bin, "nvim");
    assert_eq!(args, vec!["+123", "+normal! zz"]);
}

#[test]
fn split_editor_cmd_single_quoted_arg_with_space() {
    let (bin, args) = split_editor_cmd("nvim '+normal! zz'");
    assert_eq!(bin, "nvim");
    assert_eq!(args, vec!["+normal! zz"]);
}

#[test]
fn split_editor_cmd_double_quote_inside_single_quotes() {
    let (bin, args) = split_editor_cmd(r#"nvim '"foo"'"#);
    assert_eq!(bin, "nvim");
    assert_eq!(args, vec![r#""foo""#]);
}

#[test]
fn split_editor_cmd_unclosed_quote_is_graceful() {
    let (bin, args) = split_editor_cmd(r#"nvim +"abc"#);
    assert_eq!(bin, "nvim");
    assert_eq!(args, vec!["+abc"]);
}
