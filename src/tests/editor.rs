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
    let strategy =
        try_new_tab_command(editor, Path::new(file), 1, &emulator, false, false, None).unwrap();
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
    let strategy = try_new_tab_command(
        "nano",
        Path::new("/tmp/test.md"),
        1,
        &emulator,
        true,
        false,
        None,
    );
    assert!(strategy.is_none());
}

#[test]
fn try_new_tab_command_windows_terminal_wsl_windows_editor_returns_spawn() {
    let emulator = TerminalEmulator::WindowsTerminal;
    let strategy = try_new_tab_command(
        "notepad.exe",
        Path::new("/tmp/test.md"),
        1,
        &emulator,
        true,
        false,
        None,
    )
    .unwrap();
    assert!(matches!(strategy, LaunchStrategy::SpawnAndAssume(_)));
}

#[test]
fn try_new_tab_command_windows_terminal_non_wsl_returns_spawn() {
    let emulator = TerminalEmulator::WindowsTerminal;
    let strategy = try_new_tab_command(
        "nano",
        Path::new("/tmp/test.md"),
        1,
        &emulator,
        false,
        false,
        None,
    )
    .unwrap();
    assert!(matches!(strategy, LaunchStrategy::SpawnAndAssume(_)));
}

#[test]
fn try_new_tab_command_kitty_returns_run_and_check() {
    let emulator = TerminalEmulator::Kitty;
    let strategy = try_new_tab_command(
        "nano",
        Path::new("/tmp/test.md"),
        1,
        &emulator,
        false,
        false,
        None,
    )
    .unwrap();
    assert!(matches!(strategy, LaunchStrategy::RunAndCheck(_)));
}

#[test]
fn try_new_tab_command_gnome_returns_spawn() {
    let emulator = TerminalEmulator::GnomeTerminal;
    let strategy = try_new_tab_command(
        "vim",
        Path::new("/tmp/test.md"),
        1,
        &emulator,
        false,
        false,
        None,
    )
    .unwrap();
    assert!(matches!(strategy, LaunchStrategy::SpawnAndAssume(_)));
}

#[test]
fn try_new_tab_command_termux_returns_none() {
    let emulator = TerminalEmulator::Termux;
    let strategy = try_new_tab_command(
        "nano",
        Path::new("/tmp/test.md"),
        1,
        &emulator,
        false,
        false,
        None,
    );
    assert!(strategy.is_none());
}

#[test]
fn try_new_tab_command_unknown_returns_none() {
    let emulator = TerminalEmulator::Unknown;
    let strategy = try_new_tab_command(
        "nano",
        Path::new("/tmp/test.md"),
        1,
        &emulator,
        false,
        false,
        None,
    );
    assert!(strategy.is_none());
}

fn zellij_command(
    editor: &str,
    file: &str,
    line: usize,
    emulator: &TerminalEmulator,
) -> Vec<String> {
    let strategy =
        try_new_tab_command(editor, Path::new(file), line, emulator, false, true, None).unwrap();
    let cmd = match strategy {
        LaunchStrategy::RunAndCheck(cmd) => cmd,
        LaunchStrategy::SpawnAndAssume(_) => panic!("expected RunAndCheck for zellij"),
    };
    let mut out = vec![cmd.get_program().to_string_lossy().into_owned()];
    out.extend(cmd.get_args().map(|a| a.to_string_lossy().into_owned()));
    out
}

#[test]
fn try_new_tab_command_zellij_edits_in_place() {
    let args = zellij_command("nano", "/tmp/test.md", 1, &TerminalEmulator::Unknown);
    assert_eq!(
        args,
        vec![
            "zellij",
            "action",
            "edit",
            "--in-place",
            "--line-number",
            "1",
            "/tmp/test.md"
        ]
    );
}

#[test]
fn try_new_tab_command_zellij_wins_over_host_emulator() {
    for emulator in [
        TerminalEmulator::MacTerminal("iTerm.app".into()),
        TerminalEmulator::Kitty,
        TerminalEmulator::GnomeTerminal,
        TerminalEmulator::Unknown,
    ] {
        let args = zellij_command("nano", "/tmp/test.md", 1, &emulator);
        assert_eq!(args[0], "zellij", "host emulator won for {emulator:?}");
    }
}

#[test]
fn try_new_tab_command_zellij_passes_visible_line() {
    let args = zellij_command("hx", "/tmp/test.md", 42, &TerminalEmulator::Unknown);
    let idx = args
        .iter()
        .position(|a| a == "--line-number")
        .expect("--line-number arg missing");
    assert_eq!(args[idx + 1], "42");
}

#[test]
fn try_new_tab_command_zellij_clamps_zero_line() {
    let args = zellij_command("hx", "/tmp/test.md", 0, &TerminalEmulator::Unknown);
    let idx = args
        .iter()
        .position(|a| a == "--line-number")
        .expect("--line-number arg missing");
    assert_eq!(args[idx + 1], "1");
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

#[test]
fn format_editor_tab_title_short_name_no_truncation() {
    assert_eq!(
        format_editor_tab_title(Path::new("/tmp/readme.md"), Some(30)),
        "leaf editor: readme.md"
    );
}

#[test]
fn format_editor_tab_title_none_disables_truncation() {
    assert_eq!(
        format_editor_tab_title(Path::new("/tmp/verylongfilenamewithoutextension"), None,),
        "leaf editor: verylongfilenamewithoutextension"
    );
}

#[test]
fn format_editor_tab_title_truncates_preserving_extension() {
    let out = format_editor_tab_title(Path::new("/tmp/chapitre-1-introduction.md"), Some(25));
    assert!(
        out.starts_with("leaf editor: "),
        "unexpected prefix in {out}"
    );
    let display = &out["leaf editor: ".len()..];
    assert!(display.ends_with(".md"), "should keep extension: {display}");
    assert!(display.contains("..."), "should be truncated: {display}");
    assert!(out.chars().count() <= 25, "total len exceeds 25: {out}");
}

#[test]
fn format_editor_tab_title_hidden_file_no_extension() {
    let out = format_editor_tab_title(Path::new("/tmp/.env"), Some(30));
    assert_eq!(out, "leaf editor: .env");
}

#[test]
fn format_editor_tab_title_file_without_extension() {
    let out = format_editor_tab_title(Path::new("/tmp/README"), Some(30));
    assert_eq!(out, "leaf editor: README");
}

#[test]
fn format_editor_tab_title_unicode_boundary_safe() {
    let out = format_editor_tab_title(Path::new("/tmp/éàü-très-long-nom.md"), Some(25));
    assert!(out.starts_with("leaf editor: "));
    assert!(out.chars().count() <= 25);
}

#[test]
fn format_editor_tab_title_name_with_single_quote() {
    let out = format_editor_tab_title(Path::new("/tmp/don't-remove.md"), None);
    assert_eq!(out, "leaf editor: don't-remove.md");
}

#[test]
fn kitty_tab_title_uses_filename() {
    let strategy = try_new_tab_command(
        "nano",
        Path::new("/tmp/readme.md"),
        1,
        &TerminalEmulator::Kitty,
        false,
        false,
        None,
    )
    .unwrap();
    let cmd = match strategy {
        LaunchStrategy::RunAndCheck(cmd) => cmd,
        LaunchStrategy::SpawnAndAssume(_) => panic!("expected RunAndCheck for Kitty"),
    };
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert!(
        args.iter()
            .any(|a| a == "--tab-title=leaf editor: readme.md"),
        "missing tab title in args: {args:?}"
    );
}

#[test]
fn gnome_tab_title_uses_filename() {
    let strategy = try_new_tab_command(
        "vim",
        Path::new("/tmp/notes.md"),
        1,
        &TerminalEmulator::GnomeTerminal,
        false,
        false,
        None,
    )
    .unwrap();
    let cmd = match strategy {
        LaunchStrategy::SpawnAndAssume(cmd) => cmd,
        LaunchStrategy::RunAndCheck(_) => panic!("expected SpawnAndAssume for GnomeTerminal"),
    };
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert!(
        args.iter().any(|a| a == "--title=leaf editor: notes.md"),
        "missing tab title in args: {args:?}"
    );
}

#[test]
fn windows_terminal_tab_title_uses_filename() {
    let strategy = try_new_tab_command(
        "notepad.exe",
        Path::new("/tmp/notes.md"),
        1,
        &TerminalEmulator::WindowsTerminal,
        true,
        false,
        None,
    )
    .unwrap();
    let cmd = match strategy {
        LaunchStrategy::SpawnAndAssume(cmd) => cmd,
        LaunchStrategy::RunAndCheck(_) => panic!("expected SpawnAndAssume for WindowsTerminal"),
    };
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let title_idx = args
        .iter()
        .position(|a| a == "--title")
        .expect("--title arg missing");
    assert_eq!(args[title_idx + 1], "leaf editor: notes.md");
}

#[test]
fn apple_terminal_tab_title_uses_filename() {
    let script = mac_tab_script("nano", "/tmp/readme.md", "Apple_Terminal");
    assert!(
        script.contains("leaf editor: readme.md"),
        "script missing filename title: {script}"
    );
}

#[test]
fn apple_terminal_tab_title_escapes_single_quote() {
    let script = mac_tab_script("nano", "/tmp/don't.md", "Apple_Terminal");
    assert!(
        script.contains(r"leaf editor: don'\\''t.md"),
        "single quote not escaped for shell (after AppleScript encoding): {script}"
    );
}
