use super::App;
use crate::markdown::hash_str;
use std::time::Instant;

pub(crate) const FLASH_DURATION_MS: u64 = 1500;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditorFlash {
    Opened(String),
    NoFile,
    EditorNotFound(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WatchFlash {
    Activated,
    Deactivated,
    Stdin,
    NoFile,
    FileNotFound,
    NotActive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LinkFlash {
    Copied,
    CopyFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PathFlash {
    RelativeCopied,
    AbsoluteCopied,
    CopyFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CodeBlockFlash {
    Copied,
    CopyFailed,
    NoneVisible,
}

impl App {
    pub(crate) fn set_editor_flash(&mut self, flash: EditorFlash) {
        self.editor_flash = Some((flash, Instant::now()));
    }

    pub(crate) fn editor_flash(&self) -> Option<&(EditorFlash, Instant)> {
        self.editor_flash.as_ref()
    }

    pub(crate) fn clear_editor_flash(&mut self) {
        self.editor_flash = None;
    }

    pub(crate) fn toggle_watch(&mut self) {
        let p = match &self.filepath {
            None => {
                self.set_watch_flash(if self.filename == "stdin" {
                    WatchFlash::Stdin
                } else {
                    WatchFlash::NoFile
                });
                return;
            }
            Some(p) => p,
        };
        if !p.exists() {
            self.set_watch_flash(WatchFlash::FileNotFound);
            return;
        }
        self.watch = !self.watch;
        self.set_watch_flash(if self.watch {
            WatchFlash::Activated
        } else {
            WatchFlash::Deactivated
        });
        if self.watch {
            self.last_file_state = None;
            self.last_content_hash = hash_str(&self.source);
            self.last_hash_check = Some(Instant::now());
            self.watch_error = false;
        }
    }

    pub(crate) fn watch_flash(&self) -> Option<(&WatchFlash, &Instant)> {
        self.watch_flash.as_ref().map(|(f, t)| (f, t))
    }

    pub(crate) fn set_watch_flash(&mut self, flash: WatchFlash) {
        self.watch_flash = Some((flash, Instant::now()));
    }

    pub(crate) fn watch_flash_for_no_file(&self) -> WatchFlash {
        if self.filename == "stdin" {
            WatchFlash::Stdin
        } else {
            WatchFlash::NoFile
        }
    }

    pub(crate) fn clear_watch_flash(&mut self) {
        self.watch_flash = None;
    }

    pub(crate) fn set_config_warning(&mut self, warning: Option<String>) {
        if let Some(msg) = warning {
            self.config_flash = Some((msg, Instant::now()));
        }
    }

    pub(crate) fn config_flash(&self) -> Option<(&str, &Instant)> {
        self.config_flash.as_ref().map(|(msg, t)| (msg.as_str(), t))
    }

    pub(crate) fn clear_config_flash(&mut self) {
        self.config_flash = None;
    }

    pub(crate) fn set_link_flash(&mut self, flash: LinkFlash) {
        self.link_flash = Some((flash, Instant::now()));
    }

    pub(crate) fn link_flash(&self) -> Option<(&LinkFlash, &Instant)> {
        self.link_flash.as_ref().map(|(f, t)| (f, t))
    }

    pub(crate) fn clear_link_flash(&mut self) {
        self.link_flash = None;
    }

    pub(crate) fn set_path_flash(&mut self, flash: PathFlash) {
        self.path_flash = Some((flash, Instant::now()));
    }

    pub(crate) fn path_flash(&self) -> Option<&(PathFlash, Instant)> {
        self.path_flash.as_ref()
    }

    pub(crate) fn clear_path_flash(&mut self) {
        self.path_flash = None;
    }

    pub(crate) fn clear_reload_flash(&mut self) {
        self.reload_flash = None;
    }

    pub(crate) fn reload_flash_started(&self) -> Option<Instant> {
        self.reload_flash
    }

    pub(crate) fn set_code_block_flash(&mut self, flash: CodeBlockFlash) {
        self.code_block_flash = Some((flash, Instant::now()));
    }

    pub(crate) fn code_block_flash(&self) -> Option<(&CodeBlockFlash, &Instant)> {
        self.code_block_flash.as_ref().map(|(f, t)| (f, t))
    }
}
