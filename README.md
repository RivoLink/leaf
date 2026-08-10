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

**macOS / Linux / Android / Termux**

```bash
curl -fsSL https://raw.githubusercontent.com/RivoLink/leaf/main/scripts/install.sh | sh
```

**Windows**

```powershell
irm https://raw.githubusercontent.com/RivoLink/leaf/main/scripts/install.ps1 | iex
```

**npm**

Install the package from [npmjs.com](https://www.npmjs.com/package/@rivolink/leaf):

```bash
npm install -g @rivolink/leaf
```

**Homebrew**

Install the formula from [brew.sh](https://formulae.brew.sh/formula/leaf-markdown-viewer):

```bash
brew install leaf-markdown-viewer
```

**Cargo**

Install the crate from [crates.io](https://crates.io/crates/leaf-markdown-viewer):

```bash
cargo install leaf-markdown-viewer
```

**Scoop (Windows)**

Install the app from [scoop.sh](https://scoop.sh/#/apps?q=leaf-markdown-viewer):

```bash
scoop install leaf-markdown-viewer
```

**ArchLinux (AUR)**

Install the package from [archlinux.org](https://aur.archlinux.org/packages/leaf-markdown-viewer-bin), use an [AUR helper](https://wiki.archlinux.org/title/AUR_helpers) such as `yay`:

```bash
yay -S leaf-markdown-viewer
```

**Verify the installation**

```bash
leaf --version
```

## Update

Update an existing installation to the latest published release.

**Self**

```bash
leaf --update
```

`leaf --update` downloads the matching published asset, verifies it against the published `checksums.txt` SHA256, and then installs it.

On Windows, if replacing the running `.exe` is blocked by the OS, rerun the PowerShell installer from the install section.

**npm**

```bash
npm update -g @rivolink/leaf
```

**Homebrew**

```bash
brew upgrade leaf-markdown-viewer
```

**Cargo**

```bash
cargo install leaf-markdown-viewer --force
```

**Scoop (Windows)**

```bash
scoop update leaf-markdown-viewer
```

## Usage

```bash
# Open a Markdown file
leaf TESTING.md

# Watch mode: reloads automatically on save
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

# Pick a color theme
leaf --theme forest TESTING.md

# Set the external editor
leaf --editor nvim TESTING.md
leaf -e code TESTING.md

# Cap the content width (min: 20)
leaf --width 100 TESTING.md

# Stream Markdown from another CLI tool
claude "explain Rust lifetimes" | leaf

# Preview a local file through stdin
cat TESTING.md | leaf
```

## Inline Mode

Render Markdown directly to **stdout** without the interactive TUI:

```bash
# Render to terminal with colors
leaf --inline README.md

# Force plain text, no ANSI codes (no colors)
leaf --inline plain README.md

# Force ANSI colors even when piping
leaf --inline ansi README.md

# Set a specific width
leaf --inline 60 README.md
leaf --inline ansi:60 README.md

# Pipe from stdin
cat README.md | leaf --inline

# Use as a fzf preview
fzf --preview 'leaf --inline ansi {}'
fzf --preview 'leaf --inline ansi:$FZF_PREVIEW_COLUMNS {}'
```

## Shell Completions

Enable Tab completion for all arguments:

```bash
leaf --auto-complete
```

Supports **bash**, **zsh**, **fish**, and **PowerShell**. Restart your shell to activate.

## Vim Integration
Add the following to your `~/.vimrc` to preview the current Markdown file in a vertical split:

```vim
" Preview the current Markdown file in a vertical split using leaf
nnoremap <Leader>md :vertical botright terminal leaf -w %<CR>
```

Once added, use `\md` to open a live preview. To switch focus back to the Markdown buffer, press `Ctrl+w,h`.

## Configuration

Set default values for **theme**, **editor**, **watch** mode, **extra** file types, and more via `config.toml`:

```bash
leaf --config
```

This opens the configuration file in your editor. If the file does not exist yet, **leaf** creates it with documented defaults.

```toml
theme = "ocean"            # arctic, forest, ocean, solarized-dark, or a custom theme file
editor = "nano"            # any editor in PATH
watch = false              # auto-reload when opening a file
width = 80                 # maximum content width (min: 20, default: terminal width)
extras = ["txt", "rs"]     # extra file types shown in the picker
file-picker-width = "75%"  # width of the file picker (fuzzy + file browser)
code-line-numbers = true   # show line numbers inside fenced code blocks
tab-title-length = -1      # terminal tab title truncation (min: 20, -1: no truncation)
```

To reset the configuration to defaults:

```bash
leaf --config reset
```

To remove the configuration directory and all its contents:

```bash
leaf --config remove
```

All settings are optional. CLI arguments always take priority. See [`config.toml`](config.toml) for details.

## Open in Editor

Press `Ctrl+E` to open the current file in the configured editor.

Use `{$line}` in `editor` to open at the line currently visible in **leaf**:

```toml
editor = 'nano +{$line}'
editor = 'nvim +{$line} +"normal! zz"'
```

For further customization, `{$path}` is also available for the file path:

```toml
editor = 'code -g {$path}:{$line}'
```

Without `{$line}`, the editor opens at the top of the file.

Inside a **zellij** session, `Ctrl+E` opens the editor in a pane placed over **leaf**'s own pane (`zellij action edit --in-place`) instead of a new tab of the host terminal. Closing the pane brings **leaf** back.

On this path the editor is the one zellij is configured with (`scrollback_editor`, else `$EDITOR`), so the `editor` setting applies to GUI editors only. The visible line is always passed, so `{$line}` is not needed.

## Extra Files

Non-Markdown files can be listed in the file picker by adding their extensions to `config.toml`:

```toml
extras = ["txt", "csv", "rs", "java", "json", "yaml"]
```

Code files get syntax highlighting; text files are rendered as plain Markdown.

Any file can also be opened directly from the command line, regardless of the `extras` setting:

```bash
leaf main.rs
```

Browse and preview code files with fzf:

```bash
find . -name '*.rs' | fzf --preview 'leaf --inline ansi {}'
```

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

| Key | Action | Key | Action |
|---|---|---|---|
| `j` / `↓` | Scroll down | `?` | Show help popup |
| `k` / `↑` | Scroll up | `t` | Toggle TOC sidebar |
| `d` / PgDn | Page down (20 lines) | `p` | Show file path |
| `u` / PgUp | Page up (20 lines) | `Shift+L` | Toggle line numbers |
| `g` / Home | Top | `Shift+T` | Open theme picker |
| `G` / End | Bottom | `Shift+E` | Open editor picker |
| `1-9` / `0+1-9` | Jump / reverse jump (TOC) | `Shift+P` | Open file browser |
| `J/K` / `U/D` | Navigate TOC | `Shift+M` | Toggle mouse capture |
| `y/Y` / `c/C` | Focus code block | `Ctrl+P` | Open fuzzy picker |
| `Ctrl+L` | Go to line | `Ctrl+E` | Open in editor |
| `Ctrl+F` / `/` | Find | `Ctrl+Click` | Open link |
| `n` / `N` | Next / prev match | `Double-Click` (link) | Copy link |
| `w` | Toggle watch mode | `Double-Click` (code) | Copy code block |
| `r` | Force reload (watch mode) | `Shift+Drag` | Select text |
| `q` | Quit | `Option+Drag` | Select text (iTerm2) |

## Features

- **Live preview** : *Watch mode with automatic reload and visual feedback*.
- **File picker** : *Fuzzy Markdown picker, directory browser, and watch after selection*.
- **Editor integration** : *Open the current file in your preferred editor*.
- **Frontmatter support** : *YAML frontmatter rendered as a table (horizontal or vertical based on key count)*.
- **Rich Markdown rendering** : *Tables, lists, blockquotes, rules, bold, italic, and strikethrough*.
- **GitHub extras** : *Alert callouts, task list checkboxes, and `==mark==` text highlighting*.
- **Extra file types** : *Open any file; code files get syntax highlighting, text files render as Markdown*.
- **Syntax highlighting** : *Common aliases like `py`, `cpp`, `json`, `toml`, `ps1`, `dockerfile`*.
- **Line numbers** : *Toggle display with `Shift+L`, jump to a line with `Ctrl+L`*.
- **LaTeX support** : *Inline, block, and `latex` / `tex` code blocks rendered as formulas*.
- **Mermaid diagrams** : *`mermaid` code blocks rendered as ASCII diagrams*.
- **Clickable links** : *`Ctrl+Click` to open, double-click to copy, hover feedback*.
- **Code block interactions** : *Focus and copy with `y/Y` / `c/C`, or double-click on a block*.
- **Mouse capture** : *`Shift+M` to toggle mouse capture and let the terminal handle selection*.
- **Navigation** : *TOC sidebar, active section tracking, heading jumps, and search*.
- **Terminal UX** : *Theme picker, help popup, file path popup, mouse and keyboard support*.
- **Custom themes** : *TOML theme files inheriting from built-in presets with color overrides*.
- **Inline mode** : *Render to stdout with `--inline` for pipes and fzf previews*.
- **Shell completions** : *Tab completion for bash, zsh, fish, and PowerShell via `leaf --auto-complete`*.
- **CLI friendly** : *stdin support and `leaf --update` with SHA256 verification*.

## Typical AI Workflow

```bash
# Terminal 1: generate the file
aichat "..." > notes.md

# Terminal 2: live watch
leaf --watch notes.md
```

## Troubleshooting

### Windows: missing Visual C++ runtime

If `leaf.exe` does not start on Windows or reports a missing MSVC runtime, install the latest supported Microsoft Visual C++ Redistributable from Microsoft Learn:

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

### Windows: auto-complete execution policy error

If PowerShell reports that running scripts is disabled on this system after `leaf --auto-complete`, allow local scripts and restart PowerShell:

```powershell
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser
```

## Uninstall

The uninstall script removes the **leaf** configuration, shell auto-completion, and binary. It is interactive, so download first and run locally.

**macOS / Linux / Android / Termux**

```bash
# Download
curl -fsSL https://raw.githubusercontent.com/RivoLink/leaf/main/scripts/uninstall.sh -o uninstall.sh

# Run
sh uninstall.sh
```

**Windows**

```powershell
# Download
irm https://raw.githubusercontent.com/RivoLink/leaf/main/scripts/uninstall.ps1 -OutFile uninstall.ps1

# Run
powershell -ExecutionPolicy Bypass -File .\uninstall.ps1
```

**npm**

```bash
leaf --config remove
leaf --auto-complete remove

npm uninstall -g @rivolink/leaf
```

**Homebrew**

```bash
leaf --config remove
leaf --auto-complete remove

brew uninstall leaf-markdown-viewer
```

**Cargo**

```bash
leaf --config remove
leaf --auto-complete remove

cargo uninstall leaf-markdown-viewer
```

**Scoop (Windows)**

```bash
leaf --config remove
leaf --auto-complete remove

scoop uninstall leaf-markdown-viewer
```

## Contributors

Thanks to all contributors.

![Contributors](https://contrib.rocks/image?repo=RivoLink/leaf&t=717807600)

## Support

Contributions are welcome. Feel free to open an issue or submit a pull request.

See the [CONTRIBUTING.md](CONTRIBUTING.md) file for details.

If you like **leaf**, consider giving the project a star ⭐

## License

This project is licensed under the MIT License.

See the [LICENSE](LICENSE) file for details.
