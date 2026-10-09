use crate::diff::{parse_unified_diff, FileChange, HunkLine, WordDiffKind};

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/diff")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
}

#[test]
fn basic_single_file_single_hunk() {
    let diff = fixture("basic.diff");
    let files = parse_unified_diff(&diff);
    assert_eq!(files.len(), 1);
    let f = &files[0];
    assert_eq!(f.old_path, "src/foo.rs");
    assert_eq!(f.new_path, "src/foo.rs");
    assert!(matches!(f.change, FileChange::Modified));
    assert_eq!(f.hunks.len(), 1);
    let h = &f.hunks[0];
    assert_eq!(h.header.old_start, 1);
    assert_eq!(h.header.old_count, 3);
    assert_eq!(h.header.new_start, 1);
    assert_eq!(h.header.new_count, 3);
    assert_eq!(h.lines.len(), 4);
    assert!(matches!(h.lines[0], HunkLine::Context(ref s) if s == "fn main() {"));
    assert!(matches!(h.lines[1], HunkLine::Del(ref s) if s == "    println!(\"hello\");"));
    assert!(matches!(h.lines[2], HunkLine::Add(ref s) if s == "    println!(\"world\");"));
    assert!(matches!(h.lines[3], HunkLine::Context(ref s) if s == "}"));
}

#[test]
fn create_dev_null_marks_added() {
    let files = parse_unified_diff(&fixture("create.diff"));
    assert_eq!(files.len(), 1);
    let f = &files[0];
    assert!(
        matches!(f.change, FileChange::Added),
        "expected Added, got {:?}",
        f.change
    );
    assert_eq!(f.new_path, "src/new.rs");
    assert_eq!(f.old_path, "/dev/null");
    assert_eq!(f.hunks.len(), 1);
    assert!(f
        .hunks
        .iter()
        .flat_map(|h| h.lines.iter())
        .any(|l| matches!(l, HunkLine::Add(_))));
}

#[test]
fn delete_dev_null_marks_deleted() {
    let files = parse_unified_diff(&fixture("delete.diff"));
    assert_eq!(files.len(), 1);
    assert!(
        matches!(files[0].change, FileChange::Deleted),
        "got {:?}",
        files[0].change
    );
    assert_eq!(files[0].new_path, "/dev/null");
    assert_eq!(files[0].old_path, "src/old.rs");
}

#[test]
fn rename_with_similarity_index() {
    let files = parse_unified_diff(&fixture("rename.diff"));
    assert_eq!(files.len(), 1);
    let f = &files[0];
    match &f.change {
        FileChange::Renamed {
            from,
            to,
            similarity,
        } => {
            assert_eq!(from, "src/old_name.rs");
            assert_eq!(to, "src/new_name.rs");
            assert_eq!(*similarity, Some(92));
        }
        other => panic!("expected Renamed, got {other:?}"),
    }
    assert_eq!(f.hunks.len(), 1);

    assert_eq!(f.hunks[0].header.section.as_deref(), Some("fn helper() {"));
}

#[test]
fn mode_change_without_hunk() {
    let files = parse_unified_diff(&fixture("mode_change.diff"));
    assert_eq!(files.len(), 1);
    match &files[0].change {
        FileChange::ModeOnly { old_mode, new_mode } => {
            assert_eq!(old_mode, "100644");
            assert_eq!(new_mode, "100755");
        }
        other => panic!("expected ModeOnly, got {other:?}"),
    }
    assert!(files[0].hunks.is_empty());
}

#[test]
fn binary_files_differ_marker() {
    let files = parse_unified_diff(&fixture("binary.diff"));
    assert_eq!(files.len(), 1);
    assert!(matches!(files[0].change, FileChange::Binary));
    assert!(files[0].hunks.is_empty());
}

#[test]
fn git_binary_patch_marker() {
    let files = parse_unified_diff(&fixture("binary_patch.diff"));
    assert_eq!(files.len(), 1);
    assert!(matches!(files[0].change, FileChange::Binary));
    assert!(files[0].hunks.is_empty());
}

#[test]
fn submodule_change_recognized_via_content() {
    let files = parse_unified_diff(&fixture("submodule.diff"));
    assert_eq!(files.len(), 1);

    assert!(matches!(files[0].change, FileChange::Submodule));
    let has_subproject = files[0].hunks.iter().flat_map(|h| h.lines.iter()).any(
        |l| matches!(l, HunkLine::Add(s) | HunkLine::Del(s) if s.starts_with("Subproject commit")),
    );
    assert!(has_subproject);
}

#[test]
fn no_newline_marker_captured() {
    let files = parse_unified_diff(&fixture("no_newline.diff"));
    assert_eq!(files.len(), 1);
    let markers = files[0]
        .hunks
        .iter()
        .flat_map(|h| h.lines.iter())
        .filter(|l| matches!(l, HunkLine::NoNewline))
        .count();
    assert_eq!(markers, 2);
}

#[test]
fn no_prefix_paths_are_captured_verbatim() {
    let files = parse_unified_diff(&fixture("no_prefix.diff"));
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].old_path, "src/foo.rs");
    assert_eq!(files[0].new_path, "src/foo.rs");
    assert_eq!(files[0].hunks.len(), 1);
}

#[test]
fn multi_file_yields_multiple_diff_files() {
    let files = parse_unified_diff(&fixture("multi_file.diff"));
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].new_path, "first.rs");
    assert_eq!(files[1].new_path, "second.rs");
}

#[test]
fn empty_input_yields_empty_result() {
    assert!(parse_unified_diff("").is_empty());
}

#[test]
fn unknown_leading_noise_is_ignored() {
    let src = "some random preamble line\nnot a diff\n";
    assert!(parse_unified_diff(src).is_empty());
}

#[test]
fn hunk_short_form_counts_default_to_one() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
    let files = parse_unified_diff(src);
    let h = &files[0].hunks[0];
    assert_eq!(h.header.old_count, 1);
    assert_eq!(h.header.new_count, 1);
}

#[test]
fn diff_cc_starts_a_file_entry() {
    let src = "diff --cc merged.rs\n@@@ -1,2 -1,2 +1,2 @@@\n foo\n";
    let files = parse_unified_diff(src);
    assert_eq!(files.len(), 1);

    assert!(files[0].old_path.contains("merged.rs"));
}

#[test]
fn very_large_hunk_parses_without_panic() {
    let mut src = String::from(
        "diff --git a/big.rs b/big.rs\n--- a/big.rs\n+++ b/big.rs\n@@ -1,1000 +1,1000 @@\n",
    );
    for i in 0..1000 {
        if i % 2 == 0 {
            src.push_str(&format!(" line {i}\n"));
        } else {
            src.push_str(&format!("-line {i}\n+LINE {i}\n"));
        }
    }
    let files = parse_unified_diff(&src);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].hunks.len(), 1);
    assert!(files[0].hunks[0].lines.len() >= 1000);
}

#[test]
fn combined_diff_recognizes_at_at_at_hunk_header() {
    let files = parse_unified_diff(&fixture("combined.diff"));
    assert_eq!(files.len(), 1);
    let f = &files[0];
    assert!(f.old_path.contains("src/merge.rs"));
    assert_eq!(f.hunks.len(), 1);
    let h = &f.hunks[0];
    assert_eq!(h.header.parent_count, 2, "expected 2-parent combined hunk");
    assert!(h
        .lines
        .iter()
        .any(|l| matches!(l, HunkLine::Combined(_, _))));
}

#[test]
fn combined_diff_uniform_context_stays_as_context() {
    let files = parse_unified_diff(&fixture("combined.diff"));
    let h = &files[0].hunks[0];

    let context_count = h
        .lines
        .iter()
        .filter(|l| matches!(l, HunkLine::Context(_)))
        .count();
    assert!(
        context_count >= 1,
        "expected at least one pure-context line, got hunk lines: {:?}",
        h.lines
    );
}

#[test]
fn word_diff_line_splits_into_segments() {
    let files = parse_unified_diff(&fixture("word_diff.diff"));
    assert_eq!(files.len(), 1);
    let f = &files[0];
    assert_eq!(f.hunks.len(), 1);
    let h = &f.hunks[0];
    let word_diff_lines: Vec<_> = h
        .lines
        .iter()
        .filter_map(|l| match l {
            HunkLine::WordDiff(segs) => Some(segs.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(word_diff_lines.len(), 2);
    let first = &word_diff_lines[0];

    assert!(matches!(first[0].kind, WordDiffKind::Common));
    assert!(first
        .iter()
        .any(|s| matches!(s.kind, WordDiffKind::Del) && s.text == "a"));
    assert!(first
        .iter()
        .any(|s| matches!(s.kind, WordDiffKind::Add) && s.text == "the"));
}

#[test]
fn submodule_recursive_nested_diff_is_attached() {
    let files = parse_unified_diff(&fixture("submodule_recursive.diff"));
    assert_eq!(files.len(), 1);
    let f = &files[0];
    assert!(matches!(f.change, FileChange::Submodule));
    let nested = f
        .nested_submodule_diff
        .as_ref()
        .expect("nested submodule diff attached");
    assert_eq!(nested.len(), 1);
    assert_eq!(nested[0].new_path, "nested.rs");
    assert_eq!(nested[0].hunks.len(), 1);
}

#[test]
fn index_line_captures_blob_shas() {
    let src = "diff --git a/x b/x\n\
               index abc1234..def5678 100644\n\
               --- a/x\n\
               +++ b/x\n\
               @@ -1,1 +1,1 @@\n\
               -a\n\
               +b\n";
    let files = parse_unified_diff(src);
    assert_eq!(files.len(), 1);
    let f = &files[0];
    assert_eq!(f.old_blob_sha.as_deref(), Some("abc1234"));
    assert_eq!(f.new_blob_sha.as_deref(), Some("def5678"));
}

#[test]
fn index_line_without_mode_still_captures_shas() {
    let src = "diff --git a/y b/y\n\
               index 0123456789abcdef..fedcba9876543210\n\
               --- a/y\n\
               +++ b/y\n\
               @@ -1,1 +1,1 @@\n\
               -a\n\
               +b\n";
    let files = parse_unified_diff(src);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].old_blob_sha.as_deref(), Some("0123456789abcdef"));
    assert_eq!(files[0].new_blob_sha.as_deref(), Some("fedcba9876543210"));
}

#[test]
fn missing_index_line_leaves_blob_shas_none() {
    let src = "diff --git a/z b/z\n--- a/z\n+++ b/z\n@@ -1 +1 @@\n-a\n+b\n";
    let files = parse_unified_diff(src);
    assert_eq!(files.len(), 1);
    assert!(files[0].old_blob_sha.is_none());
    assert!(files[0].new_blob_sha.is_none());
}

#[test]
fn lossy_input_does_not_panic() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-\u{FFFD}old\n+\u{FFFD}new\n";
    let files = parse_unified_diff(src);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].hunks.len(), 1);
}
