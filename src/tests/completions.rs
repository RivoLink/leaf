use crate::cli::{parse_cli, AutoCompleteArg, AutoCompleteMode};

#[test]
fn parse_cli_accepts_auto_complete() {
    let args = vec!["leaf".to_string(), "--auto-complete".to_string()];
    let options = parse_cli(&args).unwrap();
    assert_eq!(
        options.auto_complete,
        Some(AutoCompleteArg {
            shell: None,
            mode: AutoCompleteMode::Install,
        })
    );
}

#[test]
fn parse_cli_auto_complete_with_shell() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "bash".to_string(),
    ];
    let options = parse_cli(&args).unwrap();
    assert_eq!(
        options.auto_complete,
        Some(AutoCompleteArg {
            shell: Some("bash".to_string()),
            mode: AutoCompleteMode::Install,
        })
    );
}

#[test]
fn parse_cli_auto_complete_dump() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "dump".to_string(),
    ];
    let options = parse_cli(&args).unwrap();
    assert_eq!(
        options.auto_complete,
        Some(AutoCompleteArg {
            shell: None,
            mode: AutoCompleteMode::Dump,
        })
    );
}

#[test]
fn parse_cli_auto_complete_shell_dump() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "zsh:dump".to_string(),
    ];
    let options = parse_cli(&args).unwrap();
    assert_eq!(
        options.auto_complete,
        Some(AutoCompleteArg {
            shell: Some("zsh".to_string()),
            mode: AutoCompleteMode::Dump,
        })
    );
}

#[test]
fn parse_cli_auto_complete_remove() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "remove".to_string(),
    ];
    let options = parse_cli(&args).unwrap();
    assert_eq!(
        options.auto_complete,
        Some(AutoCompleteArg {
            shell: None,
            mode: AutoCompleteMode::Remove,
        })
    );
}

#[test]
fn parse_cli_auto_complete_shell_remove() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "bash:remove".to_string(),
    ];
    let options = parse_cli(&args).unwrap();
    assert_eq!(
        options.auto_complete,
        Some(AutoCompleteArg {
            shell: Some("bash".to_string()),
            mode: AutoCompleteMode::Remove,
        })
    );
}

#[test]
fn parse_cli_auto_complete_unknown_shell_remove() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "xxx:remove".to_string(),
    ];
    assert!(parse_cli(&args).is_err());
}

#[test]
fn parse_cli_auto_complete_invalid_arg() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "invalid".to_string(),
    ];
    assert!(parse_cli(&args).is_err());
}

#[test]
fn auto_complete_rejects_with_file() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "README.md".to_string(),
    ];
    assert!(parse_cli(&args).is_err());
}

#[test]
fn auto_complete_rejects_with_watch() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "--watch".to_string(),
    ];
    assert!(parse_cli(&args).is_err());
}

#[test]
fn auto_complete_rejects_with_update() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "--update".to_string(),
    ];
    assert!(parse_cli(&args).is_err());
}

#[test]
fn auto_complete_rejects_with_config() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "--config".to_string(),
    ];
    assert!(parse_cli(&args).is_err());
}

#[test]
fn parse_cli_auto_complete_nushell() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "nushell".to_string(),
    ];
    let options = parse_cli(&args).unwrap();
    assert_eq!(
        options.auto_complete,
        Some(AutoCompleteArg {
            shell: Some("nushell".to_string()),
            mode: AutoCompleteMode::Install,
        })
    );
}

#[test]
fn parse_cli_auto_complete_nushell_dump() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "nushell:dump".to_string(),
    ];
    let options = parse_cli(&args).unwrap();
    assert_eq!(
        options.auto_complete,
        Some(AutoCompleteArg {
            shell: Some("nushell".to_string()),
            mode: AutoCompleteMode::Dump,
        })
    );
}

#[test]
fn parse_cli_auto_complete_nushell_remove() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "nushell:remove".to_string(),
    ];
    let options = parse_cli(&args).unwrap();
    assert_eq!(
        options.auto_complete,
        Some(AutoCompleteArg {
            shell: Some("nushell".to_string()),
            mode: AutoCompleteMode::Remove,
        })
    );
}

#[test]
fn auto_complete_rejects_with_theme() {
    let args = vec![
        "leaf".to_string(),
        "--auto-complete".to_string(),
        "--theme".to_string(),
        "arctic".to_string(),
    ];
    assert!(parse_cli(&args).is_err());
}
