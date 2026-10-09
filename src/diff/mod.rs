pub(crate) mod layout;
pub(crate) mod parser;

pub(crate) use layout::DiffLayout;
pub(crate) use parser::parse_unified_diff;

pub(crate) const NO_NEWLINE_MARKER: &str = "\\ No newline at end of file";

pub(crate) fn is_mode_change_with_hunks(file: &DiffFile) -> bool {
    matches!(file.change, FileChange::ModeOnly { .. }) && !file.hunks.is_empty()
}

pub(crate) fn is_rename_with_hunks(file: &DiffFile) -> bool {
    matches!(file.change, FileChange::Renamed { .. }) && !file.hunks.is_empty()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DiffFile {
    pub(crate) old_path: String,
    pub(crate) new_path: String,
    pub(crate) change: FileChange,
    pub(crate) header_lines: Vec<String>,
    pub(crate) hunks: Vec<Hunk>,
    pub(crate) nested_submodule_diff: Option<Vec<DiffFile>>,
    pub(crate) old_blob_sha: Option<String>,
    pub(crate) new_blob_sha: Option<String>,
}

impl DiffFile {
    pub(crate) fn new(old_path: String, new_path: String, header_line: &str) -> Self {
        Self {
            old_path,
            new_path,
            change: FileChange::Modified,
            header_lines: vec![header_line.to_string()],
            hunks: Vec::new(),
            nested_submodule_diff: None,
            old_blob_sha: None,
            new_blob_sha: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileChange {
    Modified,
    Added,
    Deleted,
    Renamed {
        from: String,
        to: String,
        similarity: Option<u16>,
    },
    ModeOnly {
        old_mode: String,
        new_mode: String,
    },
    Binary,
    Submodule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Hunk {
    pub(crate) header: HunkHeader,
    pub(crate) header_raw: String,
    pub(crate) lines: Vec<HunkLine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HunkHeader {
    pub(crate) old_start: u32,
    pub(crate) old_count: u32,
    pub(crate) new_start: u32,
    pub(crate) new_count: u32,
    pub(crate) section: Option<String>,
    pub(crate) parent_count: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HunkLine {
    Context(String),
    Add(String),
    Del(String),
    NoNewline,
    Combined(String, String),
    WordDiff(Vec<WordDiffSegment>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WordDiffKind {
    Common,
    Add,
    Del,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WordDiffSegment {
    pub(crate) kind: WordDiffKind,
    pub(crate) text: String,
}

pub(crate) fn strip_prefix(path: &str) -> &str {
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path)
}
