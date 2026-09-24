//! Suspend and resume: `Ctrl+Z`, `SIGTSTP` and `SIGCONT`.

use ratatui::{backend::CrosstermBackend, Terminal};

type Term = Terminal<CrosstermBackend<std::io::Stdout>>;

#[cfg(unix)]
mod imp {
    use super::Term;
    use anyhow::Result;
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::layout::Rect;
    use signal_hook::{
        consts::signal::{SIGCONT, SIGTSTP},
        flag, low_level,
    };
    use std::{
        io,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, LazyLock,
        },
    };

    static SUSPEND_REQUESTED: LazyLock<Arc<AtomicBool>> = LazyLock::new(Default::default);
    static RESUME_REQUESTED: LazyLock<Arc<AtomicBool>> = LazyLock::new(Default::default);
    // Set while a child owns the terminal, so SIGTSTP stops the whole job.
    static CHILD_RUNNING: LazyLock<Arc<AtomicBool>> = LazyLock::new(Default::default);

    pub(crate) fn install_signal_handlers() -> io::Result<()> {
        flag::register(SIGTSTP, Arc::clone(&SUSPEND_REQUESTED))?;
        flag::register_conditional_default(SIGTSTP, Arc::clone(&CHILD_RUNNING))?;
        flag::register(SIGCONT, Arc::clone(&RESUME_REQUESTED))?;
        Ok(())
    }

    /// Handles a pending `SIGTSTP` or `SIGCONT`; true if the TUI was re-entered.
    pub(crate) fn handle_signals(terminal: &mut Term, mouse_capture: bool) -> Result<bool> {
        if SUSPEND_REQUESTED.swap(false, Ordering::SeqCst) {
            suspend(terminal, mouse_capture)?;
        } else if RESUME_REQUESTED.swap(false, Ordering::SeqCst) {
            resume(terminal, mouse_capture)?;
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    /// Restores the shell's terminal, stops, and re-enters the TUI on `SIGCONT`.
    pub(crate) fn suspend(terminal: &mut Term, mouse_capture: bool) -> Result<()> {
        if mouse_capture {
            execute!(terminal.backend_mut(), DisableMouseCapture)?;
        }
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        disable_raw_mode()?;
        low_level::emulate_default_handler(SIGTSTP)?; // returns after SIGCONT
        clear_pending();
        resume(terminal, mouse_capture)
    }

    /// Runs a child that owns the terminal; `SIGTSTP` then stops us with it.
    pub(crate) fn with_foreground_child<T>(run: impl FnOnce() -> T) -> T {
        CHILD_RUNNING.store(true, Ordering::SeqCst);
        let result = run();
        CHILD_RUNNING.store(false, Ordering::SeqCst);
        clear_pending();
        result
    }

    fn clear_pending() {
        SUSPEND_REQUESTED.store(false, Ordering::SeqCst);
        RESUME_REQUESTED.store(false, Ordering::SeqCst);
    }

    fn resume(terminal: &mut Term, mouse_capture: bool) -> Result<()> {
        // Re-apply raw mode: the shell may have reset the tty while stopped.
        disable_raw_mode()?;
        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        if mouse_capture {
            execute!(terminal.backend_mut(), EnableMouseCapture)?;
        }
        // Full redraw without `Terminal::clear`, which queries the cursor.
        let size = terminal.size()?;
        terminal.resize(Rect::new(0, 0, size.width, size.height))?;
        Ok(())
    }
}

#[cfg(not(unix))]
mod imp {
    use super::Term;
    use anyhow::Result;
    use std::io;

    pub(crate) fn install_signal_handlers() -> io::Result<()> {
        Ok(())
    }

    pub(crate) fn handle_signals(_terminal: &mut Term, _mouse_capture: bool) -> Result<bool> {
        Ok(false)
    }

    pub(crate) fn suspend(_terminal: &mut Term, _mouse_capture: bool) -> Result<()> {
        Ok(())
    }

    pub(crate) fn with_foreground_child<T>(run: impl FnOnce() -> T) -> T {
        run()
    }
}

pub(crate) use imp::{handle_signals, install_signal_handlers, suspend, with_foreground_child};
