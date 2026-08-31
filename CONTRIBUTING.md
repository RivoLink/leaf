# Contributing

Thank you for your interest in contributing to `leaf`.

## Getting Started

1. Fork the repository.
2. Clone your fork:

```bash
git clone https://github.com/<your-username>/leaf.git
cd leaf
```

3. Install Rust if needed:

```bash
rustup show
```

4. Build the project:

```bash
cargo build
```

5. Run the application:

```bash
cargo run -- README.md
```

## Development Workflow

Before submitting a PR, run the full validation sequence:

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Pull Requests

1. Create a feature branch from `main`.
2. Make focused changes.
3. Keep documentation in sync when behavior changes.
4. Ensure all checks pass.
5. Open a PR with a clear description of:
   - what changed;
   - why it changed;
   - how it was tested.

## Commit Signing

The `main` branch enforces verified commits, so sign yours with GPG if you can.\
Otherwise the maintainer will re-sign them on merge, keeping you as the original author.

See [GitHub GPG signing guide](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits).

## Commit Messages

- Use clear, direct messages.
- Keep the first line short.
- Prefer the existing prefixes used in the repository when they fit.

Examples:

- `feat: custom themes`
- `chore: refactor main.rs`
- `fix: file picker alignment`
- `docs: update README.md`

## Code Style

- Follow the existing Rust style in the repository.
- Prefer small, targeted refactors over broad rewrites.
- Preserve the current terminal UX unless the change explicitly improves it.
- Keep ASCII by default unless the file already uses Unicode intentionally.

## Testing Notes

- Add regression tests when fixing rendering or parsing bugs.
- Prefer narrow, behavior-focused tests over snapshot-heavy tests.
- If a change affects terminal layout, verify both tests and manual rendering.

## Architecture

See:

- [ARCHITECTURE.md](./ARCHITECTURE.md)

## Releases

Release automation lives in:

- [.github/workflows/release-cut.yml](./.github/workflows/release-cut.yml)
- [.github/workflows/release-build.yml](./.github/workflows/release-build.yml)
- [scripts/release-cut.sh](./scripts/release-cut.sh)

Installers live in:

- [scripts/install.sh](./scripts/install.sh)
- [scripts/install.ps1](./scripts/install.ps1)

## Questions

Open an issue or discussion in the repository:

- https://github.com/RivoLink/leaf/issues
- https://github.com/RivoLink/leaf/discussions
