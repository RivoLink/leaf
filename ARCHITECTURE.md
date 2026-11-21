# Architecture

`leaf` is a single Rust binary with an interactive terminal UI and a non-interactive Markdown renderer (`--inline`). `src/main.rs` wires together input handling, application state, parsing, rendering, and terminal lifecycle.

## Component map

- `src/main.rs`: composition root. Parses CLI options, loads configuration and theme assets, resolves input and mode, creates `App`, and starts the terminal runtime.
- `src/cli.rs`: CLI argument parsing and help/version text, including diff, picker, history, config, completion, update, and inline commands.
- `src/config.rs` and `src/picker_width.rs`: config-file paths and parsing, defaults, validation, and width specifications. The config directory also owns custom themes and `history.toml`.
- `src/app/`: interactive state and operations.
  - `mod.rs` owns the shared `App` state, rendered content, caches, overlays, and optional `DiffState`.
  - `content.rs` loads and reloads documents, tracks file changes, and reparses content when needed.
  - `mode.rs` defines document/diff state and detects diff input.
  - `navigation.rs`, `diff_nav.rs`, and `diff_toggles.rs` handle document and diff navigation, layout, file tree, and preview controls.
  - `diff_preview.rs` obtains or reconstructs before/after file content for diff previews.
  - `file_picker.rs`, `fuzzy.rs`, and `io_picker.rs` implement browser/fuzzy picker state, matching, indexing, and asynchronous loading.
  - `history.rs` and `history_picker.rs` persist, load, and present recent files.
  - `comments.rs` holds review comments: comment-mode cursor and range, source-line anchors, and clipboard export.
  - `search.rs`, `goto_line.rs`, `links.rs`, `code_blocks.rs`, `theme_picker.rs`, `popups.rs`, and `flash.rs` hold interaction-specific state and behavior.
- `src/markdown/`: width-aware Markdown parsing and text layout. `mod.rs` drives `pulldown-cmark`; supporting modules render blocks, inline spans, lists, tables, links, footnotes, frontmatter, LaTeX, Mermaid, and the table of contents. `syntax.rs` and `highlight.rs` integrate Syntect; `width.rs` and `wrapping.rs` handle display width.
- `src/diff/`: Git diff data types, unified-diff parsing, and the unified/split layout choice.
- `src/render/`: Ratatui frame composition. `content.rs`, `toc.rs`, `diff_tree.rs`, `status.rs`, `popup.rs`, and `popup_picker.rs` render the main panels and overlays; `diff/` builds and draws unified, split, and preview panels.
- `src/runtime/`: Crossterm event loop, timers, resize handling, keyboard routing, diff/code-selection/comment-mode key handling (`comment_keys.rs`), and mouse interactions.
- `src/theme/`: built-in presets, custom-theme deserialization and resolution, UI/Markdown/diff colors, and the selected Syntect syntax theme.
- Supporting modules: `editor.rs` launches configured editors; `clipboard.rs` opens links and copies text; `completions.rs` installs shell completions; `inline.rs` serializes rendered lines; `terminal.rs` manages terminal state; `update.rs` downloads and verifies releases.
- `src/tests/`: in-binary unit and regression tests, organized by subsystem. Shared helpers live in `src/tests/common/`; diff fixtures live in `tests/fixtures/diff/`.

## Startup and input flow

1. `main.rs` parses arguments and handles one-shot actions such as help, version, config operations, history edit/remove/list, shell completion, and update before opening the TUI. The history picker remains an interactive action.
2. It loads `config.toml` and resolves settings. Theme selection follows `--theme` > `LEAF_THEME` > the config value > the default.
3. Input comes from a file, stdin, a directory, or a fuzzy query. With an interactive terminal and no file, Leaf opens the fuzzy picker by default; `--picker` opens the browser picker. `--last` uses file history.
4. Diff mode is forced by `--diff`, inferred from a `.diff`/`.patch` extension, or detected from a Git diff marker on the first nonblank line among the first 16 lines. `--diff` can run `git diff` for the working tree, index, or a revision, with an optional path filter.
5. Markdown documents are parsed into styled terminal lines. Non-Markdown source files with a recognized syntax are wrapped as fenced code blocks. In diff mode, the diff parser and render builders create the diff view.
6. `--inline` writes parsed lines to stdout and exits without entering the TUI. Otherwise, `main.rs` configures `App`, queues any initial picker, enters a `TerminalSession`, draws the first frame, and starts `runtime::run`.

## Application state and rendering

`AppMode` has two values: `Document` and `Diff`. Whether content is loaded is tracked separately by `App::has_content()`: before the first file is selected, the picker is the main view. Pickers and popups are independent state overlays, not additional application modes.

The Markdown parser returns a `ParseResult` containing styled `ratatui::Line` values plus the TOC, link spans and URLs, code-block ranges, and rendered/source line maps. `App` stores that result, derives searchable plain-text lines, and tracks scrolling, selection, watch state, and interaction caches.

`render::ui` lays out the content and status bar. Document mode can show the TOC sidebar; diff mode can show a file tree, unified or split content, or a before/after preview. Popup and picker rendering is layered over the active panel. `runtime::run` dispatches keyboard, mouse, and resize events, polls pending work and timers, and redraws when state changes.

## Document and diff pipelines

### Markdown

`markdown::parse_markdown_with_width` extracts YAML frontmatter, normalizes code fences, then consumes `pulldown-cmark` events. Block and inline renderers apply the active Markdown theme, wrap content to the effective width, and use Syntect for code highlighting. LaTeX and Mermaid blocks use dedicated renderers. The result includes the metadata needed by navigation, search, links, and code-block selection. Resizing or changing the preview theme reparses document content.

### Git diffs

`diff::parse_unified_diff` converts Git patch text into `DiffFile` values containing file metadata and hunks. `render::diff` builds unified lines and their file/hunk/line-number mappings; split lines are built separately. `DiffState` tracks current file, layout, tree navigation, preview visibility, and per-file preview/scroll caches. Before/after previews use Git objects or the working tree/index when available, with hunk reconstruction as a fallback.

## Background work and persistence

- Browser, fuzzy, and history pickers are queued in `App`. The runtime starts worker threads, receives results over `mpsc` channels, and installs successful results after polling. Picker loading has a 500 ms minimum display time; errors are represented as picker state. Cancelling drops the receiver; it does not forcibly stop the worker thread.
- Diff setup builds unified lines immediately. When multiple CPUs are available, split lines are precomputed on a worker thread and installed when the runtime polls the result. If no split build is pending, toggling to split builds it synchronously.
- In the UI, history recording/removal runs on worker threads, and the history picker loads entries asynchronously. Writes use an `fs2` lock and a temporary file followed by rename; failures are sent back to the UI.
- `TerminalSession` owns raw mode, the alternate screen, mouse capture, and synchronized updates. The normal exit path and `Drop` attempt to restore the terminal.

## State updates

- Opening or reloading a document replaces the source, reparses it, and refreshes derived lines, TOC, links, code-block metadata, and search data.
- In document mode, an effective-width change triggers a width-aware reparse. Diff rendering tracks the viewport separately.
- The theme picker previews built-in presets using cached parse results and can restore the original selection and content.
- Search state lives in `App`; the current match drives highlighting and scroll position.

## Tests

Tests are declared from `src/tests/mod.rs` under `#[cfg(test)]` in `src/main.rs`. They cover app state, CLI/config behavior, Markdown parsing, diff parsing/rendering, pickers, themes, terminal output, and updates.
