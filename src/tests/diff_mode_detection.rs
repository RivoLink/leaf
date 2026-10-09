use crate::app::mode::{
    detect_mode, detect_mode_from_bytes, detect_mode_from_content, is_diff_extension, AppMode,
};
use crate::app::DiffSource;
use crate::cli::{is_diff_spec, parse_cli, parse_diff_spec};
use std::path::PathBuf;

#[test]
fn detects_diff_git_prefix_bytes() {
    let buf = b"diff --git a/foo b/foo\nindex 000..111 100644\n--- a/foo\n+++ b/foo\n";
    assert_eq!(detect_mode_from_bytes(buf), AppMode::Diff);
}

#[test]
fn detects_diff_cc_prefix_bytes() {
    let buf = b"diff --cc file.txt\n";
    assert_eq!(detect_mode_from_bytes(buf), AppMode::Diff);
}

#[test]
fn detects_diff_combined_prefix_bytes() {
    let buf = b"diff --combined file.txt\n";
    assert_eq!(detect_mode_from_bytes(buf), AppMode::Diff);
}

#[test]
fn ignores_diff_signature_after_first_non_blank_line() {
    let buf = b"# Heading\n\ndiff --git a/x b/x\n";
    assert_eq!(detect_mode_from_bytes(buf), AppMode::Document);
}

#[test]
fn skips_blank_lines_bytes() {
    let buf = b"\n\n   \ndiff --git a/x b/x\n";
    assert_eq!(detect_mode_from_bytes(buf), AppMode::Diff);
}

#[test]
fn tolerates_non_utf8_bytes_after_signature() {
    let mut buf = Vec::from(&b"diff --git a/x b/x\n"[..]);
    buf.extend_from_slice(&[0xC3, 0x28, 0xFF, 0xFE]);
    assert_eq!(detect_mode_from_bytes(&buf), AppMode::Diff);
}

#[test]
fn detects_diff_from_str_content() {
    let src = "diff --git a/foo b/foo\nindex 000..111 100644\n";
    assert_eq!(detect_mode_from_content(src), AppMode::Diff);
}

#[test]
fn no_false_positive_on_changelog_with_diff_fence() {
    let src = "# Changelog\n\n## v1\n\n```diff\ndiff --git a/foo b/foo\n```\n";
    assert_eq!(detect_mode_from_content(src), AppMode::Document);
}

#[test]
fn no_false_positive_on_readme_with_yaml_frontmatter() {
    let src = "---\ntitle: leaf\n---\n\n# leaf\n\nSee `git diff --git`\n";
    assert_eq!(detect_mode_from_content(src), AppMode::Document);
}

#[test]
fn no_false_positive_on_blog_talking_about_git() {
    let src = "Some prose about `git diff --git a/x b/x` embedded inline.\n";
    assert_eq!(detect_mode_from_content(src), AppMode::Document);
}

#[test]
fn empty_input_is_document() {
    assert_eq!(detect_mode_from_bytes(b""), AppMode::Document);
    assert_eq!(detect_mode_from_content(""), AppMode::Document);
}

#[test]
fn only_blank_lines_is_document() {
    assert_eq!(detect_mode_from_bytes(b"\n\n \t\n"), AppMode::Document);
    assert_eq!(detect_mode_from_content("\n\n  \t\n"), AppMode::Document);
}

#[test]
fn detect_mode_cli_forced_wins() {
    let src = "# heading\n";
    assert_eq!(detect_mode(None, true, src), AppMode::Diff);
}

#[test]
fn detect_mode_diff_extension_wins_over_content() {
    let src = "# not really a diff\n";
    let path = PathBuf::from("foo.diff");
    assert_eq!(detect_mode(Some(&path), false, src), AppMode::Diff);
}

#[test]
fn detect_mode_patch_extension() {
    let path = PathBuf::from("foo.patch");
    assert_eq!(detect_mode(Some(&path), false, ""), AppMode::Diff);
}

#[test]
fn detect_mode_content_sniff_when_no_hint() {
    let src = "diff --git a/x b/x\n";
    assert_eq!(detect_mode(None, false, src), AppMode::Diff);
}

#[test]
fn detect_mode_md_file_with_diff_fence_stays_document() {
    let src = "# Notes\n\n```\ndiff --git a/x b/x\n```\n";
    let path = PathBuf::from("README.md");
    assert_eq!(detect_mode(Some(&path), false, src), AppMode::Document);
}

#[test]
fn is_diff_extension_matches_diff_and_patch() {
    assert!(is_diff_extension("diff"));
    assert!(is_diff_extension("patch"));
    assert!(is_diff_extension("DIFF"));
    assert!(is_diff_extension("Patch"));
    assert!(!is_diff_extension("md"));
    assert!(!is_diff_extension(""));
}

#[test]
fn is_diff_spec_accepts_source_keywords() {
    assert!(is_diff_spec("cached"));
    assert!(is_diff_spec("staged"));
    assert!(is_diff_spec("working"));
    assert!(is_diff_spec("HEAD"));
    assert!(is_diff_spec("main..topic"));
    assert!(is_diff_spec("origin/feature"));
}

#[test]
fn is_diff_spec_rejects_flags_and_empty() {
    assert!(!is_diff_spec(""));
    assert!(!is_diff_spec("--watch"));
    assert!(!is_diff_spec("-w"));
}

#[test]
fn parse_diff_spec_recognizes_cached_and_staged() {
    assert_eq!(
        parse_diff_spec("cached").unwrap().source,
        DiffSource::Cached
    );
    assert_eq!(
        parse_diff_spec("staged").unwrap().source,
        DiffSource::Cached
    );
}

#[test]
fn parse_diff_spec_recognizes_working() {
    assert_eq!(
        parse_diff_spec("working").unwrap().source,
        DiffSource::Working
    );
}

#[test]
fn parse_diff_spec_treats_other_as_ref() {
    assert_eq!(
        parse_diff_spec("HEAD~2").unwrap().source,
        DiffSource::Ref("HEAD~2".to_string())
    );
    assert_eq!(
        parse_diff_spec("main..topic").unwrap().source,
        DiffSource::Ref("main..topic".to_string())
    );
}

#[test]
fn parse_diff_spec_rejects_empty() {
    assert!(parse_diff_spec("").is_err());
    assert!(parse_diff_spec("   ").is_err());
}

#[test]
fn parse_cli_accepts_diff_no_spec() {
    let args = vec!["leaf".into(), "--diff".into()];
    let options = parse_cli(&args).unwrap();
    let spec = options.diff.unwrap();
    assert_eq!(spec.source, DiffSource::Working);
}

#[test]
fn parse_cli_accepts_diff_cached_spec() {
    let args = vec!["leaf".into(), "--diff".into(), "cached".into()];
    let options = parse_cli(&args).unwrap();
    assert_eq!(options.diff.unwrap().source, DiffSource::Cached);
}

#[test]
fn parse_cli_accepts_diff_equals_form() {
    let args = vec!["leaf".into(), "--diff=cached".into()];
    let options = parse_cli(&args).unwrap();
    assert_eq!(options.diff.unwrap().source, DiffSource::Cached);
}

#[test]
fn parse_cli_accepts_diff_with_positional() {
    let args = vec![
        "leaf".into(),
        "--diff".into(),
        "cached".into(),
        "src/foo.rs".into(),
    ];
    let options = parse_cli(&args).unwrap();
    assert_eq!(options.diff.unwrap().source, DiffSource::Cached);
    assert_eq!(options.file_arg.as_deref(), Some("src/foo.rs"));
}

#[test]
fn parse_cli_diff_ref_is_accepted() {
    let args = vec!["leaf".into(), "--diff=HEAD~2".into()];
    let options = parse_cli(&args).unwrap();
    assert_eq!(
        options.diff.unwrap().source,
        DiffSource::Ref("HEAD~2".to_string())
    );
}

#[test]
fn parse_cli_rejects_diff_with_watch() {
    let args = vec!["leaf".into(), "--diff".into(), "--watch".into()];
    let err = parse_cli(&args).unwrap_err().to_string();
    assert!(
        err.contains("--diff cannot be combined with --watch"),
        "{err}"
    );
}

#[test]
fn parse_cli_rejects_diff_with_inline() {
    let args = vec!["leaf".into(), "--diff".into(), "--inline".into()];
    let err = parse_cli(&args).unwrap_err().to_string();
    assert!(
        err.contains("--diff cannot be combined with --inline"),
        "{err}"
    );
}

#[test]
fn parse_cli_rejects_diff_with_picker() {
    let args = vec!["leaf".into(), "--diff".into(), "--picker".into()];
    let err = parse_cli(&args).unwrap_err().to_string();
    assert!(
        err.contains("--diff cannot be combined with --picker"),
        "{err}"
    );
}

#[test]
fn parse_cli_rejects_diff_with_fuzzy() {
    let args = vec!["leaf".into(), "--diff".into(), "--fuzzy".into()];
    let err = parse_cli(&args).unwrap_err().to_string();
    assert!(
        err.contains("--diff cannot be combined with --fuzzy"),
        "{err}"
    );
}
