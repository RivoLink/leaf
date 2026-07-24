use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, BeginSynchronizedUpdate, EndSynchronizedUpdate,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Write};
use std::path::Path;

pub(crate) fn format_tab_title_filename(filename: &str, max_len: usize) -> String {
    if filename.len() <= max_len || filename.chars().count() <= max_len {
        return filename.to_string();
    }
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let keep = max_len
        .saturating_sub(3)
        .saturating_sub(ext.chars().count());
    let prefix: String = filename.chars().take(keep).collect();
    format!("{prefix}...{ext}")
}

pub(crate) fn set_tab_title(filename: Option<&str>, max_filename_len: Option<usize>) {
    let mut stdout = io::stdout();
    match (filename, max_filename_len) {
        (Some(name), Some(m)) if !name.is_empty() => {
            let display = format_tab_title_filename(name, m);
            let _ = write!(stdout, "\x1b]0;leaf: {display}\x07");
        }
        (Some(name), None) if !name.is_empty() => {
            let _ = write!(stdout, "\x1b]0;leaf: {name}\x07");
        }
        _ => {
            let _ = write!(stdout, "\x1b]0;leaf\x07");
        }
    }
    let _ = stdout.flush();
}

pub(crate) struct TerminalSession {
    raw_enabled: bool,
    screen_enabled: bool,
    synchronized_update: bool,
    alternate_screen_enabled: bool,
    mouse_capture_enabled: bool,
}

pub(crate) fn cleanup_terminal_state<F, G>(
    screen_enabled: &mut bool,
    raw_enabled: &mut bool,
    mut leave_screen: F,
    mut disable_raw: G,
) -> Result<()>
where
    F: FnMut() -> Result<()>,
    G: FnMut() -> Result<()>,
{
    let mut error = None;

    if *screen_enabled {
        if let Err(err) = leave_screen() {
            error = Some(err);
        }
        *screen_enabled = false;
    }

    if *raw_enabled {
        if let Err(err) = disable_raw() {
            if error.is_none() {
                error = Some(err);
            }
        }
        *raw_enabled = false;
    }

    if let Some(err) = error {
        Err(err)
    } else {
        Ok(())
    }
}

impl TerminalSession {
    pub(crate) fn enter(stdout: &mut io::Stdout) -> Result<Self> {
        enable_raw_mode()?;
        let mut session = Self {
            raw_enabled: true,
            screen_enabled: false,
            synchronized_update: false,
            alternate_screen_enabled: false,
            mouse_capture_enabled: false,
        };
        execute!(stdout, BeginSynchronizedUpdate)?;
        session.synchronized_update = true;
        execute!(stdout, EnterAlternateScreen)?;
        session.screen_enabled = true;
        session.alternate_screen_enabled = true;
        execute!(stdout, EnableMouseCapture)?;
        session.mouse_capture_enabled = true;
        Ok(session)
    }

    pub(crate) fn finish_initial_draw(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        if self.synchronized_update {
            execute!(terminal.backend_mut(), EndSynchronizedUpdate)?;
            self.synchronized_update = false;
        }
        Ok(())
    }

    pub(crate) fn restore(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        if self.synchronized_update {
            execute!(terminal.backend_mut(), EndSynchronizedUpdate)?;
            self.synchronized_update = false;
        }
        if self.mouse_capture_enabled {
            execute!(terminal.backend_mut(), DisableMouseCapture)?;
            self.mouse_capture_enabled = false;
        }
        let alternate_screen_enabled = self.alternate_screen_enabled;
        cleanup_terminal_state(
            &mut self.screen_enabled,
            &mut self.raw_enabled,
            || {
                if alternate_screen_enabled {
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                }
                Ok(())
            },
            || {
                disable_raw_mode()?;
                Ok(())
            },
        )?;
        terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.synchronized_update {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, EndSynchronizedUpdate);
            self.synchronized_update = false;
        }
        if self.mouse_capture_enabled {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, DisableMouseCapture);
            self.mouse_capture_enabled = false;
        }
        let alternate_screen_enabled = self.alternate_screen_enabled;
        let _ = cleanup_terminal_state(
            &mut self.screen_enabled,
            &mut self.raw_enabled,
            || {
                let mut stdout = io::stdout();
                if alternate_screen_enabled {
                    execute!(stdout, LeaveAlternateScreen)?;
                }
                Ok(())
            },
            || {
                disable_raw_mode()?;
                Ok(())
            },
        );
    }
}

pub(crate) fn finish_with_restore(
    run_result: Result<()>,
    restore_result: Result<()>,
) -> Result<()> {
    match (run_result, restore_result) {
        (Err(run_err), Err(restore_err)) => {
            Err(run_err.context(format!("terminal restore also failed: {restore_err}")))
        }
        (Err(run_err), Ok(())) => Err(run_err),
        (Ok(()), Err(restore_err)) => Err(restore_err),
        (Ok(()), Ok(())) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: usize = 15;

    #[test]
    fn format_tab_title_filename_short_unchanged() {
        assert_eq!(format_tab_title_filename("readme.md", MAX), "readme.md");
        assert_eq!(
            format_tab_title_filename("notes-2026.md", MAX),
            "notes-2026.md"
        );
        assert_eq!(format_tab_title_filename("script.rs", MAX), "script.rs");
        assert_eq!(format_tab_title_filename("stdin", MAX), "stdin");
        assert_eq!(format_tab_title_filename("README", MAX), "README");
        assert_eq!(format_tab_title_filename(".gitignore", MAX), ".gitignore");
        assert_eq!(format_tab_title_filename(".env", MAX), ".env");
    }

    #[test]
    fn format_tab_title_filename_at_limit_unchanged() {
        let name = "abcdefghijklmno";
        assert_eq!(name.chars().count(), MAX);
        assert_eq!(format_tab_title_filename(name, MAX), name);
    }

    #[test]
    fn format_tab_title_filename_truncates_with_extension() {
        assert_eq!(
            format_tab_title_filename("chapitre-1-introduction.md", MAX),
            "chapitre-1...md"
        );
        assert_eq!(
            format_tab_title_filename("verylongfilename.markdown", MAX),
            "very...markdown"
        );
    }

    #[test]
    fn format_tab_title_filename_md_output_is_exactly_max_len() {
        let out = format_tab_title_filename("chapitre-1-introduction.md", MAX);
        assert_eq!(out.chars().count(), MAX);
    }

    #[test]
    fn format_tab_title_filename_truncates_without_extension() {
        assert_eq!(
            format_tab_title_filename("verylongfilenamewithoutextension", MAX),
            "verylongfile..."
        );
    }

    #[test]
    fn format_tab_title_filename_utf8_boundary_safe() {
        let name = "résumé-tres-long-fichier.md";
        let out = format_tab_title_filename(name, MAX);
        assert!(out.ends_with("...md"));
        assert!(out.chars().count() <= MAX);
    }
}
