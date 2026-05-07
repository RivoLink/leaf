# Architecture

`leaf` is a terminal Markdown previewer built around a small set of focused modules:

- `src/main.rs`
  - entrypoint
  - loads CLI options
  - reads the initial document or opens the file picker
  - initializes terminal + syntax/theme assets

- `src/app/`
  - `mod.rs` — central runtime state: document content, TOC, search, watch, editor config, mode detection (`has_content()`)
  - `file_picker.rs` — fuzzy and browser picker state, async loading via thread + mpsc channel, queue/pending lifecycle
  - `fuzzy.rs` — fuzzy matching algorithm and directory sort helpers
  - `search.rs` — search state and match tracking
  - `theme_picker.rs` — theme picker state with preview cache

- `src/markdown/`
  - `mod.rs` — Markdown parsing and render preparation (headings, lists, blockquotes, code blocks, LaTeX)
  - `latex.rs` — LaTeX-to-Unicode conversion: `unicodeit` + postprocessing for `\frac`, `\sqrt`, `^{}`, `_{}`
  - `toc.rs` — TOC extraction and normalization
  - `tables.rs` — table rendering with alignment support
  - `width.rs` — width-aware helpers
  - `wrapping.rs` — line wrapping for constrained widths

- `src/render/`
  - `mod.rs` — TUI layout orchestration with `ratatui`
  - `content.rs` — main content panel and status bar rendering
  - `popup.rs` — popup rendering for help, file picker, theme picker, editor picker, picker loading/failed states
  - `status.rs` — status bar construction (brand, filename, search, watch, shortcuts, percentage)
  - `toc.rs` — TOC sidebar rendering

- `src/runtime.rs`
  - event loop
  - keyboard/mouse handling with mode-aware branching (help → picker_loading → picker_failed → file_picker → theme_picker → editor_picker → search → normal)
  - picker queue processing and poll loop
  - watch polling
  - resize-driven render width synchronization

- `src/theme.rs`
  - UI and Markdown theme presets
  - active theme preset selection
  - syntect theme mapping

- `src/editor.rs`
  - editor detection, classification (terminal vs GUI), and launch

- `src/cli.rs`
  - command-line parsing
  - usage/version text

- `src/terminal.rs`
  - raw mode / alternate screen lifecycle
  - terminal restore guarantees

- `src/update.rs`
  - self-update: asset download, SHA256 verification, and binary replacement

- `src/one_shot.rs`
  - non-interactive stdout rendering for `--render`

- `src/tests/`
  - `app.rs` — app state and mode detection tests
  - `file_picker.rs` — picker opening, fuzzy matching, sorting, truncation
  - `editor.rs` — editor detection and classification
  - `markdown.rs` — rendering regression tests
  - `render.rs` — table and code block border alignment
  - `theme.rs` — theme picker preview and restore
  - `update.rs` — release asset matching and checksum verification

## Execution flow

1. `main.rs` parses CLI options.
2. A document is loaded from:
   - a file argument, or
   - `stdin`, or
   - the file picker if no input is provided interactively.
3. In `--render` mode, `markdown/` parses width-aware content and `one_shot.rs` writes it to stdout before any terminal session starts.
4. Otherwise, `markdown/` parses the source into rendered lines + TOC.
5. `App` stores the state and caches.
6. `runtime.rs` runs the event loop:
   - processes pending picker queue → spawns loading thread
   - polls picker loading → installs results when ready
   - handles input events through mode-aware branching
7. `render/` draws each frame from `App`.

## Application modes

- **Initial mode** (`!app.has_content()`): no file loaded, picker is the main view. Quit shortcuts exit the app.
- **Preview mode** (`app.has_content()`): file loaded via argument, stdin, or picker selection. Quit shortcuts in pickers close the popup and return to the preview.
- **One-shot render mode** (`--render`): document content is rendered to stdout and the process exits without creating an `App` or entering the terminal runtime.

## Picker lifecycle

1. `queue_fuzzy_file_picker()` / `queue_file_picker()` sets `PendingPicker`
2. Main loop calls `start_pending_picker_loading()` → spawns thread, creates `mpsc::channel`
3. `poll_picker_loading()` does non-blocking `try_recv()` each tick (50ms)
4. Thread completes → result installed via `install_loaded_file_picker()`
5. Cancel: `cancel_picker_loading()` resets state to `Idle`, `Receiver` is dropped, thread finishes naturally

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
