use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use crate::cli::{AutoCompleteArg, AutoCompleteMode};

const PS1_COMPLETION: &str = include_str!("../completions/leaf.ps1");
const ZSH_COMPLETION: &str = include_str!("../completions/leaf.zsh");
const BASH_COMPLETION: &str = include_str!("../completions/leaf.bash");
const FISH_COMPLETION: &str = include_str!("../completions/leaf.fish");
const NU_COMPLETION: &str = include_str!("../completions/leaf.nu");

enum Shell {
    Pwsh,
    Zsh,
    Bash,
    Fish,
    Nushell,
}

impl Shell {
    fn name(&self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::Pwsh => "powershell",
            Shell::Nushell => "nushell",
        }
    }
}

fn completion_filename(shell: &Shell) -> &'static str {
    match shell {
        Shell::Bash => "leaf.bash",
        Shell::Zsh => "_leaf",
        Shell::Fish => "leaf.fish",
        Shell::Pwsh => "leaf.ps1",
        Shell::Nushell => "leaf.nu",
    }
}

fn source_line_for(shell: &Shell, path: &std::path::Path) -> Option<String> {
    match shell {
        Shell::Bash | Shell::Zsh => Some(format!("source {}", path.display())),
        Shell::Pwsh => Some(format!(". {}", path.display())),
        Shell::Fish | Shell::Nushell => None,
    }
}

fn detect_shell() -> Result<Shell> {
    if let Ok(shell) = std::env::var("SHELL") {
        let basename = std::path::Path::new(&shell)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        match basename {
            "zsh" => return Ok(Shell::Zsh),
            "bash" => return Ok(Shell::Bash),
            "fish" => return Ok(Shell::Fish),
            "nu" => return Ok(Shell::Nushell),
            _ => {}
        }
    }

    #[cfg(target_os = "windows")]
    return Ok(Shell::Pwsh);

    #[cfg(not(target_os = "windows"))]
    {
        for (path, shell) in [
            ("/bin/zsh", Shell::Zsh),
            ("/bin/bash", Shell::Bash),
            ("/bin/fish", Shell::Fish),
            ("/bin/nu", Shell::Nushell),
            ("/usr/bin/nu", Shell::Nushell),
        ] {
            if std::path::Path::new(path).exists() {
                return Ok(shell);
            }
        }
        bail!("Cannot detect shell. Set $SHELL to bash, zsh, fish, or nu")
    }
}

fn completion_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("APPDATA").context("Cannot determine APPDATA directory")?;
        Ok(PathBuf::from(base).join("leaf").join("completions"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").context("Cannot determine HOME directory")?;
        Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("leaf")
            .join("completions"))
    }
}

fn fish_completion_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("Cannot determine HOME directory")?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("fish")
        .join("completions"))
}

fn nushell_completion_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("APPDATA").context("Cannot determine APPDATA directory")?;
        Ok(PathBuf::from(base).join("nushell").join("autoload"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").context("Cannot determine HOME directory")?;
        Ok(PathBuf::from(home)
            .join(".config")
            .join("nushell")
            .join("autoload"))
    }
}

fn write_completion(dir: &std::path::Path, filename: &str, content: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("Cannot create directory: {}", dir.display()))?;
    let path = dir.join(filename);
    std::fs::write(&path, content)
        .with_context(|| format!("Cannot write completion file: {}", path.display()))?;
    Ok(path)
}

fn rc_path(shell: &Shell) -> Result<PathBuf> {
    match shell {
        Shell::Zsh => {
            let home = std::env::var("HOME").context("Cannot determine HOME directory")?;
            Ok(PathBuf::from(home).join(".zshrc"))
        }
        Shell::Bash => {
            let home = std::env::var("HOME").context("Cannot determine HOME directory")?;
            Ok(PathBuf::from(home).join(".bashrc"))
        }
        Shell::Pwsh | Shell::Fish | Shell::Nushell => {
            bail!("No RC file for this shell")
        }
    }
}

#[cfg(target_os = "windows")]
fn pwsh_profile_paths() -> Result<Vec<PathBuf>> {
    let base = std::env::var("USERPROFILE").context("Cannot determine USERPROFILE directory")?;
    let base = PathBuf::from(base).join("Documents");
    Ok(vec![
        base.join("PowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
        base.join("WindowsPowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
    ])
}

fn add_source_line(rc: &std::path::Path, line: &str) -> Result<bool> {
    if let Some(parent) = rc.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let content = std::fs::read_to_string(rc).unwrap_or_default();
    if content_has_line(&content, line) {
        return Ok(false);
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(rc)
        .with_context(|| format!("Cannot open {}", rc.display()))?;
    use std::io::Write;
    if !content.is_empty() && !content.ends_with('\n') {
        writeln!(file)?;
    }
    writeln!(file, "{line}")?;
    Ok(true)
}

fn parse_shell(name: &str) -> Result<Shell> {
    match name {
        "bash" => Ok(Shell::Bash),
        "zsh" => Ok(Shell::Zsh),
        "fish" => Ok(Shell::Fish),
        "powershell" => Ok(Shell::Pwsh),
        "nushell" => Ok(Shell::Nushell),
        _ => bail!("Unknown shell: '{name}'"),
    }
}

fn completion_content(shell: &Shell) -> &'static str {
    match shell {
        Shell::Bash => BASH_COMPLETION,
        Shell::Zsh => ZSH_COMPLETION,
        Shell::Fish => FISH_COMPLETION,
        Shell::Pwsh => PS1_COMPLETION,
        Shell::Nushell => NU_COMPLETION,
    }
}

pub(crate) fn run_auto_complete(arg: &AutoCompleteArg) -> Result<()> {
    let shell = match &arg.shell {
        Some(name) => parse_shell(name)?,
        None => detect_shell()?,
    };

    match arg.mode {
        AutoCompleteMode::Dump => {
            print!("{}", completion_content(&shell));
            Ok(())
        }
        AutoCompleteMode::Remove => remove_completions(&shell),
        AutoCompleteMode::Install => install_completions(&shell),
    }
}

fn check_shell_os_compat(shell: &Shell) -> Result<()> {
    #[cfg(target_os = "windows")]
    if !matches!(shell, Shell::Pwsh | Shell::Nushell) {
        bail!(
            "Shell '{}' is not supported. Use 'powershell' or 'nushell' instead.",
            shell.name()
        );
    }
    #[cfg(not(target_os = "windows"))]
    if matches!(shell, Shell::Pwsh) {
        bail!("Shell 'powershell' is not supported. Use bash, zsh, fish, or nushell.");
    }
    Ok(())
}

fn install_completions(shell: &Shell) -> Result<()> {
    check_shell_os_compat(shell)?;
    let content = completion_content(shell);
    let filename = completion_filename(shell);

    match shell {
        Shell::Pwsh => {
            let dest = write_completion(&completion_dir()?, filename, content)?;
            println!("Completion file installed: {}", dest.display());

            #[cfg(target_os = "windows")]
            {
                let source_line = source_line_for(shell, &dest).expect("pwsh has source line");
                for rc in pwsh_profile_paths()? {
                    if add_source_line(&rc, &source_line)? {
                        println!("Added to {}", rc.display());
                    } else {
                        println!("Already configured in {}", rc.display());
                    }
                }
                println!("\nRestart PowerShell to activate completions.");
            }
        }
        Shell::Zsh | Shell::Bash => {
            let dest = write_completion(&completion_dir()?, filename, content)?;
            println!("Completion file installed: {}", dest.display());

            let source_line = source_line_for(shell, &dest).expect("bash/zsh has source line");
            let rc = rc_path(shell)?;
            if add_source_line(&rc, &source_line)? {
                println!("Added to {}", rc.display());
            } else {
                println!("Already configured in {}", rc.display());
            }
            println!("\nRestart your shell or run: source {}", rc.display());
        }
        Shell::Fish => {
            let dest = write_completion(&fish_completion_dir()?, filename, content)?;
            println!("Completion file installed: {}", dest.display());
            println!("\nCompletions are available in new fish sessions automatically.");
        }
        Shell::Nushell => {
            let dest = write_completion(&nushell_completion_dir()?, filename, content)?;
            println!("Completion file installed: {}", dest.display());
            println!("\nRestart nushell to activate (requires 0.94+ for autoload).");
        }
    }

    Ok(())
}

fn remove_completions(shell: &Shell) -> Result<()> {
    check_shell_os_compat(shell)?;

    let plan = RemovalPlan::compute(shell)?;
    if plan.is_empty() {
        println!("Nothing to remove for {}.", shell.name());
        return Ok(());
    }

    if !crate::config::confirm(&format!("Remove {} completions?", shell.name()))? {
        println!("Remove cancelled.");
        return Ok(());
    }

    for path in &plan.files {
        std::fs::remove_file(path).with_context(|| format!("Cannot remove {}", path.display()))?;
        println!("Removed completion file: {}", path.display());
    }
    for (rc, line) in &plan.rc_lines {
        if remove_source_line(rc, line)? {
            println!("Removed source line from {}", rc.display());
        }
    }
    Ok(())
}

struct RemovalPlan {
    files: Vec<PathBuf>,
    rc_lines: Vec<(PathBuf, String)>,
}

impl RemovalPlan {
    fn is_empty(&self) -> bool {
        self.files.is_empty() && self.rc_lines.is_empty()
    }

    fn compute(shell: &Shell) -> Result<Self> {
        let mut files = Vec::new();
        let mut rc_lines = Vec::new();
        let filename = completion_filename(shell);

        match shell {
            Shell::Pwsh => {
                let path = completion_dir()?.join(filename);
                if path.exists() {
                    files.push(path.clone());
                }
                #[cfg(target_os = "windows")]
                {
                    let source_line = source_line_for(shell, &path).expect("pwsh has source line");
                    for rc in pwsh_profile_paths()? {
                        if rc_contains_line(&rc, &source_line)? {
                            rc_lines.push((rc, source_line.clone()));
                        }
                    }
                }
            }
            Shell::Zsh | Shell::Bash => {
                let path = completion_dir()?.join(filename);
                let source_line = source_line_for(shell, &path).expect("bash/zsh has source line");
                if path.exists() {
                    files.push(path);
                }
                let rc = rc_path(shell)?;
                if rc_contains_line(&rc, &source_line)? {
                    rc_lines.push((rc, source_line));
                }
            }
            Shell::Fish => {
                let path = fish_completion_dir()?.join(filename);
                if path.exists() {
                    files.push(path);
                }
            }
            Shell::Nushell => {
                let path = nushell_completion_dir()?.join(filename);
                if path.exists() {
                    files.push(path);
                }
            }
        }

        Ok(RemovalPlan { files, rc_lines })
    }
}

fn content_has_line(content: &str, line: &str) -> bool {
    let needle = line.trim();
    content.lines().any(|l| l.trim() == needle)
}

fn rc_contains_line(rc: &std::path::Path, line: &str) -> Result<bool> {
    let Ok(content) = std::fs::read_to_string(rc) else {
        return Ok(false);
    };
    Ok(content_has_line(&content, line))
}

fn remove_source_line(rc: &std::path::Path, line: &str) -> Result<bool> {
    let Ok(content) = std::fs::read_to_string(rc) else {
        return Ok(false);
    };
    let needle = line.trim();
    let mut removed = false;
    let filtered: Vec<&str> = content
        .lines()
        .filter(|l| {
            if l.trim() == needle {
                removed = true;
                false
            } else {
                true
            }
        })
        .collect();
    if !removed {
        return Ok(false);
    }
    let mut new_content = filtered.join("\n");
    if content.ends_with('\n') && !new_content.is_empty() {
        new_content.push('\n');
    }
    std::fs::write(rc, new_content).with_context(|| format!("Cannot write {}", rc.display()))?;
    Ok(true)
}
