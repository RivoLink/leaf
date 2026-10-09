use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) struct ScratchRepo {
    dir: PathBuf,
    saved_cwd: Option<PathBuf>,
}

impl ScratchRepo {
    pub(crate) fn new(prefix: &str) -> Option<Self> {
        if Command::new("git").arg("--version").output().is_err() {
            return None;
        }
        let mut dir = std::env::temp_dir();
        if dir.as_os_str().is_empty() || !dir.exists() {
            dir = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
        }
        let pid = std::process::id();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        dir.push(format!("{prefix}-{pid}-{ts}"));
        if std::fs::create_dir_all(&dir).is_err() {
            return None;
        }
        let saved_cwd = std::env::current_dir().ok();
        if std::env::set_current_dir(&dir).is_err() {
            let _ = std::fs::remove_dir_all(&dir);
            return None;
        }
        let repo = Self { dir, saved_cwd };
        if repo.run(&["init", "-q"]).is_none()
            || repo.run(&["config", "user.email", "t@t"]).is_none()
            || repo.run(&["config", "user.name", "t"]).is_none()
            || repo.run(&["config", "commit.gpgsign", "false"]).is_none()
        {
            return None;
        }
        Some(repo)
    }

    pub(crate) fn run(&self, args: &[&str]) -> Option<Output> {
        let out = Command::new("git").args(args).output().ok()?;
        if !out.status.success() {
            eprintln!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            return None;
        }
        Some(out)
    }

    pub(crate) fn write_file(&self, name: &str, content: &str) -> std::io::Result<()> {
        std::fs::write(self.dir.join(name), content)
    }

    pub(crate) fn commit(&self, path: &str, message: &str) -> Option<()> {
        self.run(&["add", path])?;
        self.run(&["commit", "-q", "-m", message])?;
        Some(())
    }

    #[allow(dead_code)]
    pub(crate) fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for ScratchRepo {
    fn drop(&mut self) {
        if let Some(c) = &self.saved_cwd {
            let _ = std::env::set_current_dir(c);
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
