use super::{
    strip_prefix, DiffFile, FileChange, Hunk, HunkHeader, HunkLine, WordDiffKind, WordDiffSegment,
};

const GITLINK_MODE: &str = "160000";

pub(crate) fn parse_unified_diff(src: &str) -> Vec<DiffFile> {
    let mut parser = Parser::default();
    for line in src.lines() {
        parser.push_line(line);
    }
    parser.finish()
}

#[derive(Default)]
struct Parser {
    files: Vec<DiffFile>,
    current: Option<DiffFile>,
    current_hunk: Option<Hunk>,
    submodule_nested_buf: Option<String>,
}

impl Parser {
    fn push_line(&mut self, line: &str) {
        if let Some(buf) = self.submodule_nested_buf.as_mut() {
            buf.push_str(line);
            buf.push('\n');
            return;
        }

        if let Some(rest) = line.strip_prefix("diff --git ") {
            self.flush_hunk();
            self.flush_file();
            let (a, b) = split_diff_git_paths(rest);
            self.current = Some(DiffFile::new(a, b, line));
            return;
        }

        if line.starts_with("diff --cc ") || line.starts_with("diff --combined ") {
            self.flush_hunk();
            self.flush_file();
            let path = line.split_whitespace().nth(2).unwrap_or("").to_string();
            self.current = Some(DiffFile::new(path.clone(), path, line));
            return;
        }

        if self.current.is_none() {
            return;
        }

        if let Some(hunk) = self.current_hunk.as_mut() {
            let parent_count = hunk.header.parent_count;
            if let Some(hunk_line) = classify_hunk_line(line, parent_count) {
                hunk.lines.push(hunk_line);
                return;
            }
            self.flush_hunk();
        }

        if line.starts_with("@@") {
            if let Some((header, header_raw)) = parse_hunk_header(line) {
                let expected_lines = (header.old_count + header.new_count) as usize;
                self.current_hunk = Some(Hunk {
                    header,
                    header_raw,
                    lines: Vec::with_capacity(expected_lines),
                });
            }
            return;
        }

        if is_submodule_marker_line(line) {
            if let Some(current) = self.current.as_mut() {
                current.change = FileChange::Submodule;
                current.header_lines.push(line.to_string());
            }
            self.submodule_nested_buf = Some(String::new());
            return;
        }

        self.handle_file_metadata(line);
    }

    fn flush_submodule_nested(&mut self) {
        if let Some(buf) = self.submodule_nested_buf.take() {
            let nested = parse_unified_diff(&buf);
            if let Some(current) = self.current.as_mut() {
                if !nested.is_empty() {
                    current.nested_submodule_diff = Some(nested);
                }
            }
        }
    }

    fn handle_file_metadata(&mut self, line: &str) {
        let Some(current) = self.current.as_mut() else {
            return;
        };
        current.header_lines.push(line.to_string());

        if let Some(rest) = line.strip_prefix("--- ") {
            let path = strip_prefix(rest.trim_end()).to_string();
            if rest.trim_end() == "/dev/null" {
                current.change = FileChange::Added;
            }
            current.old_path = path;
            return;
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            let path = strip_prefix(rest.trim_end()).to_string();
            if rest.trim_end() == "/dev/null" {
                current.change = FileChange::Deleted;
            }
            current.new_path = path;
            return;
        }
        if let Some(rest) = line.strip_prefix("similarity index ") {
            let similarity = rest.trim_end_matches('%').trim().parse::<u16>().ok();
            promote_to_rename(current, |change| {
                if let FileChange::Renamed { similarity: s, .. } = change {
                    *s = similarity;
                }
            });
            return;
        }
        if let Some(rest) = line.strip_prefix("rename from ") {
            let from = rest.trim().to_string();
            promote_to_rename(current, |change| {
                if let FileChange::Renamed { from: f, .. } = change {
                    *f = from.clone();
                }
            });

            current.old_path = from;
            return;
        }
        if let Some(rest) = line.strip_prefix("rename to ") {
            let to = rest.trim().to_string();
            promote_to_rename(current, |change| {
                if let FileChange::Renamed { to: t, .. } = change {
                    *t = to.clone();
                }
            });
            current.new_path = to;
            return;
        }
        if let Some(rest) = line.strip_prefix("old mode ") {
            let old_mode = rest.trim().to_string();
            if let FileChange::ModeOnly {
                old_mode: existing, ..
            } = &mut current.change
            {
                *existing = old_mode;
            } else if matches!(current.change, FileChange::Modified) {
                current.change = FileChange::ModeOnly {
                    old_mode,
                    new_mode: String::new(),
                };
            }
            return;
        }
        if let Some(rest) = line.strip_prefix("new mode ") {
            let new_mode = rest.trim().to_string();
            if let FileChange::ModeOnly {
                new_mode: existing, ..
            } = &mut current.change
            {
                *existing = new_mode;
            } else if matches!(current.change, FileChange::Modified) {
                current.change = FileChange::ModeOnly {
                    old_mode: String::new(),
                    new_mode,
                };
            }
            return;
        }
        if line.starts_with("Binary files ") && line.ends_with(" differ") {
            current.change = FileChange::Binary;
            return;
        }
        if line.starts_with("GIT binary patch") {
            current.change = FileChange::Binary;
            return;
        }
        if line.starts_with("Subproject commit ") {
            current.change = FileChange::Submodule;
            return;
        }
        if let Some(rest) = line.strip_prefix("index ") {
            let mut parts = rest.split_whitespace();
            let payload = parts.next().unwrap_or("");
            let mode = parts.next().unwrap_or("");
            if let Some((old_sha, new_sha)) = payload.split_once("..") {
                if !old_sha.is_empty() {
                    current.old_blob_sha = Some(old_sha.to_string());
                }
                if !new_sha.is_empty() {
                    current.new_blob_sha = Some(new_sha.to_string());
                }
            }
            if mode == GITLINK_MODE && matches!(current.change, FileChange::Modified) {
                current.change = FileChange::Submodule;
            }
            return;
        }

        if line.starts_with("new file mode ") && matches!(current.change, FileChange::Modified) {
            current.change = FileChange::Added;
        }
        if line.starts_with("deleted file mode ") && matches!(current.change, FileChange::Modified)
        {
            current.change = FileChange::Deleted;
        }
    }

    fn flush_hunk(&mut self) {
        if let Some(hunk) = self.current_hunk.take() {
            if let Some(current) = self.current.as_mut() {
                current.hunks.push(hunk);
            }
        }
    }

    fn flush_file(&mut self) {
        self.flush_submodule_nested();
        if let Some(file) = self.current.take() {
            self.files.push(file);
        }
    }

    fn finish(mut self) -> Vec<DiffFile> {
        self.flush_hunk();
        self.flush_file();
        self.files
    }
}

fn is_submodule_marker_line(line: &str) -> bool {
    if let Some(rest) = line.strip_prefix("Submodule ") {
        return rest.contains("..") && rest.trim_end().ends_with(':');
    }
    false
}

fn classify_hunk_line(line: &str, parent_count: u8) -> Option<HunkLine> {
    if line == super::NO_NEWLINE_MARKER {
        return Some(HunkLine::NoNewline);
    }

    if parent_count >= 2 {
        let n = parent_count as usize;
        if line.is_empty() {
            return Some(HunkLine::Context(String::new()));
        }

        let mut chars = line.chars();
        let mut prefix = String::with_capacity(n);
        for _ in 0..n {
            match chars.next() {
                Some(c) if c == '+' || c == '-' || c == ' ' => prefix.push(c),
                Some('\\') if prefix.is_empty() => return Some(HunkLine::NoNewline),
                _ => return None,
            }
        }
        let content: String = chars.as_str().to_string();

        if prefix.chars().all(|c| c == ' ') {
            return Some(HunkLine::Context(content));
        }
        return Some(HunkLine::Combined(prefix, content));
    }
    let mut chars = line.chars();
    match chars.next() {
        Some('+') => Some(HunkLine::Add(chars.as_str().to_string())),
        Some('-') => Some(HunkLine::Del(chars.as_str().to_string())),
        Some(' ') => Some(HunkLine::Context(chars.as_str().to_string())),
        Some('\\') => Some(HunkLine::NoNewline),

        None => Some(HunkLine::Context(String::new())),
        _ => {
            if let Some(segments) = try_parse_word_diff(line) {
                return Some(HunkLine::WordDiff(segments));
            }
            None
        }
    }
}

fn try_parse_word_diff(line: &str) -> Option<Vec<WordDiffSegment>> {
    let has_del = line.contains("[-");
    let has_add = line.contains("{+");
    if !has_del && !has_add {
        return None;
    }
    let bytes = line.as_bytes();
    let mut segments: Vec<WordDiffSegment> = Vec::new();
    let mut common_start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'[' && bytes[i + 1] == b'-' {
            if let Some(end) = find_marker_end(bytes, i + 2, b'-', b']') {
                if i > common_start {
                    segments.push(WordDiffSegment {
                        kind: WordDiffKind::Common,
                        text: line[common_start..i].to_string(),
                    });
                }
                segments.push(WordDiffSegment {
                    kind: WordDiffKind::Del,
                    text: line[i + 2..end].to_string(),
                });
                i = end + 2;
                common_start = i;
                continue;
            }
        }
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'+' {
            if let Some(end) = find_marker_end(bytes, i + 2, b'+', b'}') {
                if i > common_start {
                    segments.push(WordDiffSegment {
                        kind: WordDiffKind::Common,
                        text: line[common_start..i].to_string(),
                    });
                }
                segments.push(WordDiffSegment {
                    kind: WordDiffKind::Add,
                    text: line[i + 2..end].to_string(),
                });
                i = end + 2;
                common_start = i;
                continue;
            }
        }
        i += 1;
    }
    if segments.is_empty() {
        return None;
    }
    if common_start < bytes.len() {
        segments.push(WordDiffSegment {
            kind: WordDiffKind::Common,
            text: line[common_start..].to_string(),
        });
    }
    Some(segments)
}

fn find_marker_end(bytes: &[u8], from: usize, close1: u8, close2: u8) -> Option<usize> {
    let mut j = from;
    while j + 1 < bytes.len() {
        if bytes[j] == close1 && bytes[j + 1] == close2 {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn parse_hunk_header(line: &str) -> Option<(HunkHeader, String)> {
    let at_count = line.chars().take_while(|&c| c == '@').count();
    if at_count < 2 {
        return None;
    }
    let inner = &line[at_count..];

    let close = "@".repeat(at_count);
    let close_idx = inner.find(close.as_str())?;
    let range_part = &inner[..close_idx];
    let tail = &inner[close_idx + at_count..];
    let range = range_part.trim();

    let mut old_last: Option<(u32, u32)> = None;
    let mut minus_count: u8 = 0;
    let mut new_range: Option<(u32, u32)> = None;
    for tok in range.split_whitespace() {
        if let Some(rest) = tok.strip_prefix('-') {
            old_last = Some(parse_range_pair(rest));
            minus_count = minus_count.saturating_add(1);
        } else if let Some(rest) = tok.strip_prefix('+') {
            new_range = Some(parse_range_pair(rest));
        }
    }
    let (old_start, old_count) = old_last?;
    let (new_start, new_count) = new_range?;
    let section = {
        let trimmed = tail.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    };

    let parent_count = if minus_count >= 2 {
        minus_count
    } else if at_count >= 3 {
        (at_count - 1) as u8
    } else {
        1
    };
    Some((
        HunkHeader {
            old_start,
            old_count,
            new_start,
            new_count,
            section,
            parent_count,
        },
        line.to_string(),
    ))
}

fn parse_range_pair(s: &str) -> (u32, u32) {
    let (a, b) = match s.split_once(',') {
        Some((a, b)) => (a, Some(b)),
        None => (s, None),
    };
    let start = a.parse::<u32>().unwrap_or(0);
    let count = match b {
        Some(b) => b.parse::<u32>().unwrap_or(1),
        None => 1,
    };
    (start, count)
}

fn split_diff_git_paths(rest: &str) -> (String, String) {
    if let Some(a_end) = quoted_end(rest) {
        let a = &rest[..a_end];
        let after = rest[a_end..].trim_start();
        let b_end = quoted_end(after).unwrap_or(after.len());
        let b = &after[..b_end];
        return (strip_prefix(a).to_string(), strip_prefix(b).to_string());
    }

    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.len() >= 2 {
        let a = tokens[tokens.len() - 2];
        let b = tokens[tokens.len() - 1];
        return (strip_prefix(a).to_string(), strip_prefix(b).to_string());
    }
    (String::new(), String::new())
}

fn quoted_end(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.first().copied() != Some(b'"') {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

fn promote_to_rename(file: &mut DiffFile, mutator: impl FnOnce(&mut FileChange)) {
    if !matches!(file.change, FileChange::Renamed { .. }) {
        let from = file.old_path.clone();
        let to = file.new_path.clone();
        file.change = FileChange::Renamed {
            from,
            to,
            similarity: None,
        };
    }
    mutator(&mut file.change);
}
