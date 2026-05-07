<p align="center">
  <img src="images/logo-wordmark.svg" alt="leaf" width="360" />
</p>

<p align="center">
  Terminal Markdown previewer — GUI-like experience.
</p>

<p align="center">
  <img src="images/preview.png" alt="leaf" width="710px" /><br>
  <sub>See more screenshots in the <a href="demo/README.md">features</a> demo</sub>
</p>

## Install

Install the latest published binary.

macOS / Linux / Android / Termux:

```bash
curl -fsSL https://raw.githubusercontent.com/RivoLink/leaf/main/scripts/install.sh | sh
```

Windows:

```powershell
irm https://raw.githubusercontent.com/RivoLink/leaf/main/scripts/install.ps1 | iex
```

npm:

```bash
npm install -g @rivolink/leaf
```

ArchLinux (AUR):

Use an [AUR helper](https://wiki.archlinux.org/title/AUR_helpers), such as `yay`:

```bash
yay -S leaf-markdown-viewer
```

Verify the installation:

```bash
leaf --version
```

## Update

Update an existing installation to the latest published release.

Self:

```bash
leaf --update
```

`leaf --update` downloads the matching published asset, verifies it against the published `checksums.txt` SHA256, and then installs it.

On Windows, if replacing the running `.exe` is blocked by the OS, rerun the PowerShell installer from the install section.

npm:

```bash
npm update -g @rivolink/leaf
```

## Build

Clone the repository:

```bash
git clone https://github.com/RivoLink/leaf.git
cd leaf
```

Build the release binary locally:

```bash
cargo build --release
```

Create a local bin directory if needed and symlink `leaf` into it:

```bash
mkdir -p ~/.local/bin
ln -sf "$(pwd)/target/release/leaf" ~/.local/bin/leaf
```

If `~/.local/bin` is not already on your `PATH`, add it to `~/.bashrc` or `~/.zshrc`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Usage

```bash
# Open a Markdown file
leaf TESTING.md

# Watch mode — reloads automatically on save
leaf --watch TESTING.md
leaf -w TESTING.md

# Open the fuzzy Markdown picker
leaf

# Open the classic directory browser picker
leaf --picker

# Open the fuzzy Markdown picker, then watch the selected file
leaf -w

# Open the classic directory browser picker, then watch the selected file
leaf -w --picker

# Open a dash-prefixed filename
leaf -- -notes.md

# Stream Markdown from another CLI tool
claude "explain Rust lifetimes" | leaf

# Preview a local file through stdin
cat TESTING.md | leaf

# Render Markdown once and print it to stdout
leaf --render TESTING.md
cat TESTING.md | leaf --render

# Force styled or plain one-shot output
leaf --render --ansi TESTING.md
leaf --render --plain TESTING.md

# Render at a fixed width, useful for redirects and scripts
leaf --render --width 100 TESTING.md

```

In `--render` mode, leaf uses the terminal width when stdout is a terminal and 80 columns when stdout is redirected or piped. Use `--width <COLS>` to override that width explicitly.

## Configuration

Set default values for theme, editor, and watch mode via `config.toml`:

```bash
leaf --config
```

This opens the configuration file in your editor. If the file does not exist yet, leaf creates it with documented defaults.

```toml
theme = "ocean"      # arctic, forest, ocean, solarized-dark, or a custom theme file
editor = "nano"      # any editor in PATH
watch = false        # auto-reload when opening a file
```

All settings are optional. CLI arguments always take priority. See [`config.toml`](config.toml) for details.

## Custom Themes

Create a `.toml` file that inherits from a built-in theme and overrides specific colors:

```toml
theme = "/path/to/custom-theme.toml"
```

Relative paths are resolved from the config file directory.

```toml
# custom-theme.toml
base = "ocean"
syntax = "base16-ocean.dark"

[ui]
content_bg = "#282828"
toc_accent = "#fe8019"

[markdown]
text = "#ebdbb2"
heading_1 = "#fabd2f"
```

See [`gruvbox.toml`](gruvbox.toml) for a complete example with all available color keys.

## Keybindings

| Key | Action |
|---|---|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `d` / PgDn | Page down (20 lines) |
| `u` / PgUp | Page up (20 lines) |
| `g` / Home | Top |
| `G` / End | Bottom |
| `t` | Toggle TOC sidebar |
| `Shift+Sel` | Select text |
| `Shift+T` | Open theme picker |
| `Shift+E` | Open editor picker |
| `Shift+P` | Open file browser |
| `Ctrl+E` | Open in editor |
| `Ctrl+P` | Open fuzzy picker |
| `Ctrl+F` / `/` | Find |
| `n` / `N` | Next / prev match |
| `?` | Show help popup |
| `r` | Force reload (watch mode) |
| `q` | Quit |

## Features

- **Live preview** — Watch mode with automatic reload and visual feedback.
- **File picker** — Fuzzy Markdown picker, directory browser, and watch after selection.
- **Editor integration** — Open the current file in your preferred editor.
- **Frontmatter support** — YAML frontmatter rendered as a table (horizontal or vertical based on key count).
- **Rich Markdown rendering** — Tables, lists, blockquotes, rules, bold, italic, and strikethrough.
- **Syntax highlighting** — Common aliases like `py`, `cpp`, `json`, `toml`, `ps1`, `dockerfile`.
- **LaTeX support** — Inline, block, and `latex` / `tex` code blocks rendered as formulas.
- **Navigation** — TOC sidebar, active section tracking, heading jumps, and search.
- **Terminal UX** — Theme picker, help popup, file path popup, mouse and keyboard support.
- **CLI friendly** — stdin support, one-shot stdout rendering, and `leaf --update` with SHA256 verification.

## Typical AI Workflow

```bash
# Terminal 1: generate the file
aichat "..." > notes.md

# Terminal 2: live watch
leaf --watch notes.md
```

## Troubleshooting

### Windows: missing Visual C++ runtime

If `leaf.exe` does not start on Windows and reports a missing MSVC runtime, install the latest supported Microsoft Visual C++ Redistributable from Microsoft Learn:

- https://learn.microsoft.com/fr-fr/cpp/windows/latest-supported-vc-redist?view=msvc-170

Direct download for the latest supported **X64** Microsoft Visual C++ Redistributable:

- https://aka.ms/vc14/vc_redist.x64.exe

For `leaf-windows-x86_64.exe`, the relevant package is the latest supported **X64** Visual C++ v14 Redistributable.

### Windows: update or file replacement error

If `leaf --update` fails on Windows with an error about replacing, renaming, or writing `leaf.exe`, the running executable was likely locked by the OS.

Close any terminal session still running `leaf`, then rerun the PowerShell installer from the install section:

```powershell
irm https://raw.githubusercontent.com/RivoLink/leaf/main/scripts/install.ps1 | iex
```

## Uninstall

macOS / Linux / Android / Termux:

```bash
rm -f ~/.local/bin/leaf
```

Windows:

```powershell
Remove-Item "$env:LOCALAPPDATA\Programs\leaf\leaf.exe" -Force
```

npm:

```bash
npm uninstall -g @rivolink/leaf
```

## Contributors

Thanks to all contributors.

![Contributors](https://stg.contrib.rocks/image?repo=RivoLink/leaf)

## Support

Contributions are welcome. Feel free to open an issue or submit a pull request.

See the [CONTRIBUTING.md](CONTRIBUTING.md) file for details.

If you like **leaf**, consider giving the project a star ⭐

## License

This project is licensed under the MIT License.

See the [LICENSE](LICENSE) file for details.
