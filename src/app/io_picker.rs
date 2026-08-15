use super::file_picker::{
    FilePickerEntry, FilePickerMode, PickerIndexResult, PickerIndexTruncation, PickerLoadState,
};
use super::App;
use std::{fs, path::PathBuf, sync::mpsc, thread, time::Instant};
use syntect::parsing::SyntaxSet;

const MAX_FUZZY_PICKER_DIRS_VISITED: usize = 5_000;
const MAX_FUZZY_PICKER_FILES_INDEXED: usize = 10_000;
const MAX_FUZZY_PICKER_INDEX_DURATION: std::time::Duration = std::time::Duration::from_secs(5);

impl App {
    pub(crate) fn is_markdown_extension(ext: &str) -> bool {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "md" | "markdown" | "mdown" | "mkd"
        )
    }

    pub(crate) fn has_code_syntax(ext: &str, ss: &SyntaxSet) -> bool {
        crate::markdown::resolve_syntax(ext, ss).name != ss.find_syntax_plain_text().name
    }

    pub(crate) fn fence_wrap(src: &str, ext: &str) -> String {
        format!("````{ext}\n{src}\n````")
    }

    pub(crate) fn wrap_as_code_block(src: String, ext: &str, ss: &SyntaxSet) -> (String, bool) {
        if Self::is_markdown_extension(ext) || !Self::has_code_syntax(ext, ss) {
            (src, false)
        } else {
            (Self::fence_wrap(&src, ext), true)
        }
    }

    pub(super) fn is_accepted_path(path: &std::path::Path, extras: &[String]) -> bool {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) if Self::is_markdown_extension(ext) => true,
            Some(ext) => extras.iter().any(|e| e.eq_ignore_ascii_case(ext)),
            None => false,
        }
    }

    pub(super) fn build_file_picker_entries(
        dir: &std::path::Path,
        extras: &[String],
    ) -> std::io::Result<Vec<FilePickerEntry>> {
        let mut entries = Vec::new();

        if let Some(parent) = dir.parent() {
            entries.push(FilePickerEntry::new("..".to_string(), parent.to_path_buf()));
        }

        let mut dirs = Vec::new();
        let mut files = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            let name = entry.file_name().to_string_lossy().to_string();

            if file_type.is_dir() {
                dirs.push(FilePickerEntry::new(format!("{name}/"), path));
            } else if file_type.is_file() && Self::is_accepted_path(&path, extras) {
                files.push(FilePickerEntry::new(name, path));
            }
        }

        dirs.sort_by(|left, right| left.label_lower().cmp(right.label_lower()));
        files.sort_by(|left, right| left.label_lower().cmp(right.label_lower()));
        entries.extend(dirs);
        entries.extend(files);
        Ok(entries)
    }

    pub(super) fn build_fuzzy_file_picker_entries(
        dir: &std::path::Path,
        extras: &[String],
    ) -> std::io::Result<PickerIndexResult> {
        let mut entries = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        let started_at = Instant::now();
        let mut dirs_visited = 0usize;
        let mut files_indexed = 0usize;
        let mut truncated = None;

        while let Some(current_dir) = stack.pop() {
            if started_at.elapsed() >= MAX_FUZZY_PICKER_INDEX_DURATION {
                truncated = Some(PickerIndexTruncation::Time);
                break;
            }
            if dirs_visited >= MAX_FUZZY_PICKER_DIRS_VISITED {
                truncated = Some(PickerIndexTruncation::Directory);
                break;
            }
            dirs_visited += 1;

            let mut dirs = Vec::new();
            let mut files = Vec::new();

            let read_dir = match fs::read_dir(&current_dir) {
                Ok(read_dir) => read_dir,
                Err(err) => {
                    if current_dir == dir {
                        return Err(err);
                    }
                    continue;
                }
            };

            for entry in read_dir {
                if started_at.elapsed() >= MAX_FUZZY_PICKER_INDEX_DURATION {
                    truncated = Some(PickerIndexTruncation::Time);
                    break;
                }
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };
                let path = entry.path();
                let file_type = match entry.file_type() {
                    Ok(file_type) => file_type,
                    Err(_) => continue,
                };

                if file_type.is_dir() {
                    let name = entry.file_name();
                    if super::fuzzy::is_ignored_fuzzy_picker_dir_name(
                        name.to_string_lossy().as_ref(),
                    ) {
                        continue;
                    }
                    dirs.push(path);
                    continue;
                }

                if file_type.is_file() && Self::is_accepted_path(&path, extras) {
                    if files_indexed >= MAX_FUZZY_PICKER_FILES_INDEXED {
                        truncated = Some(PickerIndexTruncation::File);
                        break;
                    }
                    let label = path
                        .strip_prefix(dir)
                        .unwrap_or(&path)
                        .display()
                        .to_string();
                    files.push(FilePickerEntry::new(label, path));
                    files_indexed += 1;
                }
            }

            files.sort_by(|left, right| {
                super::fuzzy::fuzzy_entry_sort_key(left)
                    .cmp(&super::fuzzy::fuzzy_entry_sort_key(right))
            });
            dirs.sort_by_key(|path| super::fuzzy::fuzzy_directory_sort_key(dir, path));

            entries.extend(files);
            if truncated.is_some() {
                break;
            }
            dirs.reverse();
            stack.extend(dirs);
        }

        Ok(PickerIndexResult { entries, truncated })
    }

    pub(super) fn refresh_file_picker_matches(&mut self) {
        if self.is_browser_file_picker() {
            self.file_picker.filtered = (0..self.file_picker.entries.len()).collect();
            self.file_picker.match_positions = vec![Vec::new(); self.file_picker.filtered.len()];
            self.file_picker.index = self
                .file_picker
                .index
                .min(self.file_picker.filtered.len().saturating_sub(1));
            return;
        }

        let query = self.file_picker.query.trim().to_lowercase();
        if query.is_empty() {
            self.file_picker.filtered = (0..self.file_picker.entries.len()).collect();
            self.file_picker.match_positions = vec![Vec::new(); self.file_picker.filtered.len()];
            self.file_picker.index = self
                .file_picker
                .index
                .min(self.file_picker.filtered.len().saturating_sub(1));
            return;
        }

        let mut filtered = self
            .file_picker
            .entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                super::fuzzy::fuzzy_match(entry, &query).map(|(score, positions)| {
                    (
                        idx,
                        score,
                        entry.path_depth(),
                        entry.file_name_lower(),
                        entry.label_lower(),
                        positions,
                    )
                })
            })
            .collect::<Vec<_>>();

        filtered.sort_by(
            |(left_idx, left_score, left_depth, left_name, left_label, _),
             (right_idx, right_score, right_depth, right_name, right_label, _)| {
                left_score
                    .cmp(right_score)
                    .then_with(|| left_depth.cmp(right_depth))
                    .then_with(|| left_name.cmp(right_name))
                    .then_with(|| left_label.cmp(right_label))
                    .then_with(|| left_idx.cmp(right_idx))
            },
        );

        self.file_picker.filtered = filtered.iter().map(|(idx, ..)| *idx).collect();
        self.file_picker.match_positions = filtered
            .into_iter()
            .map(|(_, _, _, _, _, positions)| positions)
            .collect();
        if self.file_picker.filtered.is_empty()
            || self.file_picker.index >= self.file_picker.filtered.len()
        {
            self.file_picker.index = 0;
        }
    }

    pub(super) fn install_loaded_file_picker(
        &mut self,
        dir: PathBuf,
        mode: FilePickerMode,
        result: PickerIndexResult,
    ) -> bool {
        self.file_picker.open = true;
        self.file_picker.mode = mode;
        self.file_picker.dir = dir;
        self.file_picker.entries = result.entries;
        self.file_picker.index = 0;
        self.file_picker.truncation = if mode == FilePickerMode::Fuzzy {
            result.truncated
        } else {
            None
        };
        self.refresh_file_picker_matches();
        true
    }

    pub(crate) fn start_pending_picker_loading(&mut self) -> bool {
        use super::file_picker::PendingPicker;

        if !self.has_pending_picker() || !matches!(self.picker_load_state, PickerLoadState::Idle) {
            return false;
        }

        let pending = std::mem::replace(&mut self.pending_picker, PendingPicker::None);
        self.file_picker.query.clear();
        let (mode, dir) = match pending {
            PendingPicker::Browser(dir) => (FilePickerMode::Browser, dir),
            PendingPicker::Fuzzy(dir, query) => {
                self.file_picker.query = query.unwrap_or_default();
                (FilePickerMode::Fuzzy, dir)
            }
            PendingPicker::History => {
                let (tx, rx) = mpsc::channel();
                thread::spawn(move || {
                    let entries = super::history::load_history();
                    let _ = tx.send(entries);
                });
                self.picker_load_state = PickerLoadState::LoadingHistory {
                    started_at: Instant::now(),
                    receiver: rx,
                    pending_result: None,
                };
                return true;
            }
            PendingPicker::None => return false,
        };

        let worker_dir = dir.clone();
        let extras = self.file_picker.extras.clone();
        let (tx, rx) = mpsc::channel();
        crate::runtime::debug_log(
            self.debug_input,
            &format!("picker_loading spawn mode={mode:?} dir={}", dir.display()),
        );
        thread::spawn(move || {
            let result = match mode {
                FilePickerMode::Browser => Self::build_file_picker_entries(&worker_dir, &extras)
                    .map(|entries| PickerIndexResult {
                        entries,
                        truncated: None,
                    }),
                FilePickerMode::Fuzzy => {
                    Self::build_fuzzy_file_picker_entries(&worker_dir, &extras)
                }
            };
            let _ = tx.send(result);
        });

        self.picker_load_state = PickerLoadState::Loading {
            mode,
            dir,
            started_at: Instant::now(),
            receiver: rx,
            pending_result: None,
        };
        true
    }
}
