# Architecture

`leaf` is a terminal Markdown previewer built around a small set of focused modules:

- `src/main.rs`
  - entrypoint
  - loads CLI options
  - reads the initial document or opens the file picker
  - initializes terminal + syntax/theme assets

- `src/app/`
  - `mod.rs`  :  central runtime state: document content, TOC, search, watch, editor config, mode detection (`has_content()`)
  - `content.rs`  :  document loading, reload, reparse, modification detection
  - `navigation.rs`  :  scroll, TOC jump, numkey cycle, reverse mode
  - `goto_line.rs`  :  go-to-line mode (`Ctrl+L`): draft input, target validation, error state, contextual scroll offset
  - `flash.rs`  :  flash notification state (editor, watch, config, link, reload)
  - `code_blocks.rs`  :  code block selection, focus cycling, and clipboard copy
  - `popups.rs`  :  help, path popup, editor picker state and methods
  - `links.rs`  :  link detection, hover tracking, link span mapping
  - `file_picker.rs`  :  fuzzy and browser picker state, queue/pending lifecycle
  - `io_picker.rs`  :  filesystem scanning, async loading via thread + mpsc channel, fuzzy matching
  - `fuzzy.rs`  :  fuzzy matching algorithm and directory sort helpers
  - `history.rs`  :  file history persistence (`history.toml`), async record via fs2 lock + atomic write
  - `history_picker.rs`  :  history picker state and lifecycle (queue/load/close/activate)
  - `search.rs`  :  search state and match tracking
  - `theme_picker.rs`  :  theme picker state with preview cache

- `src/markdown/`
  - `mod.rs`  :  core Markdown parsing loop and render preparation
  - `blocks.rs`  :  block-level rendering (headings, code blocks, blockquotes, rules, LaTeX/Mermaid blocks)
  - `spans.rs`  :  inline style handling (bold, italic, code, LaTeX, links)
  - `markers.rs`  :  custom inline markers (`==mark==`): open/close delimiters, themed styles, padding
  - `lists.rs`  :  list rendering (ordered, unordered, nested)
  - `highlight.rs`  :  search match highlighting across spans
  - `syntax.rs`  :  syntect code highlighting and language resolution
  - `fences.rs`  :  code fence normalization (nested fences, tilde fences)
  - `links.rs`  :  link span detection and construction
  - `tables.rs`  :  table construction, event handling, and rendering
  - `table_layout.rs`  :  table cell sizing, wrapping, and alignment algorithms
  - `latex.rs`  :  LaTeX-to-Unicode conversion: `unicodeit` + postprocessing for `\frac`, `\sqrt`, `^{}`, `_{}`
  - `mermaid.rs`  :  Mermaid diagram ASCII rendering
  - `frontmatter.rs`  :  YAML frontmatter extraction and key-value parsing
  - `toc.rs`  :  TOC extraction and normalization
  - `width.rs`  :  width-aware helpers
  - `wrapping.rs`  :  line wrapping for constrained widths

- `src/render/`
  - `mod.rs`  :  TUI layout orchestration with `ratatui`
  - `content.rs`  :  main content panel rendering
  - `popup.rs`  :  popup rendering for help, theme picker, path display
  - `popup_picker.rs`  :  popup rendering for file picker, editor picker, loading/failed states
  - `status.rs`  :  status bar construction (brand, filename, search, watch, shortcuts, percentage)
  - `toc.rs`  :  TOC sidebar rendering

- `src/runtime/`
  - `mod.rs`  :  event loop, polling, timers, resize synchronization
  - `keyboard.rs`  :  keyboard handling with mode-aware branching (popups, pickers, goto-line, search, normal)
  - `mouse.rs`  :  mouse handling (scroll, click, double-click, scrollbar drag, link hover)

- `src/theme/`
  - `mod.rs`  :  theme types, global state, preset selection API
  - `presets.rs`  :  built-in theme definitions (Arctic, Forest, OceanDark, SolarizedDark)
  - `resolution.rs`  :  theme resolution, color parsing, custom theme file loading
  - `serde.rs`  :  theme deserialization, color visitor, override macros

- `src/editor.rs`
  - editor detection, classification (terminal vs GUI), and launch

- `src/cli.rs`
  - command-line parsing
  - usage/version text

- `src/config.rs`
  - `config.toml` loading and creation (`--config`, `--config reset`)
  - `LeafConfig` defaults (theme, editor, watch, width, extras) with CLI overrides

- `src/completions.rs`
  - shell completion installation (`--auto-complete`)
  - shell detection and embedded bash/zsh/fish/PowerShell scripts

- `src/clipboard.rs`
  - cross-platform URL opening (`open`, `xdg-open`, `cmd start`)
  - clipboard copy via OSC 52 escape sequence (base64)

- `src/inline.rs`
  - non-interactive stdout rendering (`--inline`)
  - ANSI/plain format resolution and line wrapping
  - ratatui Style-to-ANSI escape code serialization

- `src/terminal.rs`
  - raw mode / alternate screen lifecycle
  - terminal restore guarantees

- `src/update.rs`
  - self-update: asset download, SHA256 verification, and binary replacement

- `src/tests/`
  - `app.rs`  :  app state, search, and mode detection tests
  - `app_code_blocks.rs`  :  code block selection, focus cycling, and copy tests
  - `file_picker.rs`  :  picker opening, browser mode, queued transitions
  - `file_fuzzy.rs`  :  fuzzy matching, scoring, filtering, truncation
  - `markdown_lists.rs`  :  list rendering regression tests
  - `markdown_list_blocks.rs`  :  list-embedded block rendering tests (code, blockquote)
  - `markdown_tables.rs`  :  table rendering regression tests
  - `markdown_blocks.rs`  :  headings, TOC, blockquotes, code blocks, rules
  - `markdown_embedded.rs`  :  LaTeX and Mermaid rendering tests
  - `markdown_links.rs`  :  link detection and search highlight tests
  - `toc.rs`  :  TOC extraction, normalization, active section tracking, and display level tests
  - `editor.rs`  :  editor detection and classification
  - `render.rs`  :  table and code block border alignment
  - `theme.rs`  :  theme picker preview and restore
  - `config.rs`  :  configuration parsing tests
  - `completions.rs`  :  auto-complete argument parsing and completion script installation tests
  - `inline.rs`  :  inline spec parsing, format resolution, and write_lines tests
  - `update.rs`  :  release asset matching and checksum verification

## Execution flow

1. `main.rs` parses CLI options, loads `config.toml`, and handles standalone commands (`--auto-complete`, `--config`, `--update`) before any document is opened.
2. A document is loaded from:
   - a file argument, or
   - `stdin`, or
   - the file picker if no input is provided interactively.
3. `markdown/` parses the source into rendered lines + TOC.
4. If `--inline` is active, `inline.rs` writes lines to stdout and exits.
5. `App` stores the state and caches.
6. `runtime.rs` runs the event loop:
   - processes the pending picker queue and spawns the loading thread
   - polls picker loading, installing results when ready
   - handles input events through mode-aware branching
7. `render/` draws each frame from `App`.

## Application modes

- **Initial mode** (`!app.has_content()`): no file loaded, picker is the main view. Quit shortcuts exit the app.
- **Preview mode** (`app.has_content()`): file loaded via argument, stdin, or picker selection. Quit shortcuts in pickers close the popup and return to the preview.

## Picker lifecycle

1. `queue_fuzzy_file_picker()` / `queue_file_picker()` / `queue_history_picker()` sets `PendingPicker`
2. Main loop calls `start_pending_picker_loading()`, which spawns the thread and creates an `mpsc::channel`
3. `poll_picker_loading()` does non-blocking `try_recv()` each tick (50ms), honoring a minimum 500ms loading state for uniform UX
4. Once the thread completes, the result is installed via `install_loaded_file_picker()` or `install_loaded_history_picker()`
5. Cancel / close: `cancel_picker_loading()` / `close_history_picker()` resets state to `Idle`, `Receiver` is dropped, thread finishes naturally

## Important state transitions

- document reload / open:
  - source changes
  - rendered lines and TOC are rebuilt
  - caches are refreshed

- resize:
  - effective render width is recomputed
  - Markdown is reparsed width-aware

- theme preview:
  - previewed content is reparsed and cached per preset
  - `Esc` restores the original theme

- search:
  - query state lives in `App`
  - active match drives highlight + scroll position
