use std::io::Write;
use std::process::{Command, Stdio};

pub(crate) fn open_url(url: &str) -> bool {
    let cmd: &[&str] = match std::env::consts::OS {
        "macos" => &["open", url],
        "windows" => &["cmd", "/c", "start", "", url],
        _ => &["xdg-open", url],
    };
    Command::new(cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

pub(crate) fn copy_to_clipboard(text: &str) -> bool {
    let b64 = base64_encode(text.as_bytes());
    print!("\x1b]52;c;{b64}\x07");
    let _ = std::io::stdout().flush();

    if std::env::var("TERMUX_VERSION").is_ok()
        || !has_display_server(std::env::consts::OS, |name| std::env::var_os(name))
    {
        return true;
    }

    let candidates: &[&[&str]] = match std::env::consts::OS {
        "macos" => &[&["pbcopy"]],
        "windows" => &[&["clip.exe"]],
        _ => &[
            &["wl-copy"],
            &["xclip", "-selection", "clipboard"],
            &["xsel", "--clipboard", "--input"],
            &["termux-clipboard-set"],
        ],
    };
    for cmd in candidates {
        if try_pipe_to_command(cmd, text).is_ok() {
            return true;
        }
    }
    false
}

fn has_display_server(os: &str, var: impl Fn(&str) -> Option<std::ffi::OsString>) -> bool {
    matches!(os, "macos" | "windows")
        || ["DISPLAY", "WAYLAND_DISPLAY"]
            .iter()
            .any(|name| var(name).is_some_and(|value| !value.is_empty()))
}

fn try_pipe_to_command(cmd: &[&str], text: &str) -> std::io::Result<()> {
    let mut child = Command::new(cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("command failed"))
    }
}

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::has_display_server;
    use std::ffi::OsString;

    #[test]
    fn display_server_needs_a_non_empty_display_variable_on_linux() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| OsString::from(value))
            }
        };
        assert!(!has_display_server("linux", env(&[])));
        assert!(!has_display_server("linux", env(&[("DISPLAY", "")])));
        assert!(has_display_server("linux", env(&[("DISPLAY", ":0")])));
        assert!(has_display_server(
            "linux",
            env(&[("WAYLAND_DISPLAY", "wayland-0")])
        ));
        assert!(has_display_server("macos", env(&[])));
        assert!(has_display_server("windows", env(&[])));
    }
}
