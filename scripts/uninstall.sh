#!/usr/bin/env sh
set -u

err() {
    printf '%s\n' "$*" >&2
}

confirm() {
    answer=""
    printf '%s (y/N): ' "$1"
    read -r answer || true
    case "$answer" in
        y|Y|yes|YES) return 0 ;;
        *) return 1 ;;
    esac
}

BIN="$(command -v leaf 2>/dev/null || true)"

if [ -z "$BIN" ] || [ ! -x "$BIN" ]; then
    err "leaf not found in PATH. Nothing to uninstall."
    exit 1
fi

TARGET="$BIN"
if [ -L "$BIN" ]; then
    link="$(readlink "$BIN")"
    case "$link" in
        /*) TARGET="$link" ;;
        *) TARGET="$(dirname "$BIN")/$link" ;;
    esac
fi

echo "Uninstalling leaf ($BIN)..."

"$BIN" --config remove || true

"$BIN" --auto-complete remove || true

case "$TARGET" in
    */Cellar/*|*/homebrew/*|*/linuxbrew/*)
        echo "Detected Homebrew installation."
        if confirm "Run 'brew uninstall leaf-markdown-viewer'?"; then
            brew uninstall leaf-markdown-viewer
        fi
        exit 0
        ;;
    */.cargo/bin/*)
        echo "Detected Cargo installation."
        if confirm "Run 'cargo uninstall leaf-markdown-viewer'?"; then
            cargo uninstall leaf-markdown-viewer
        fi
        exit 0
        ;;
    */pnpm/*)
        echo "Detected pnpm installation."
        if confirm "Run 'pnpm uninstall -g @rivolink/leaf'?"; then
            pnpm uninstall -g @rivolink/leaf
        fi
        exit 0
        ;;
    */.yarn/*|*/yarn/global/*)
        echo "Detected yarn installation."
        if confirm "Run 'yarn global remove @rivolink/leaf'?"; then
            yarn global remove @rivolink/leaf
        fi
        exit 0
        ;;
    */node_modules/*)
        echo "Detected npm installation."
        if confirm "Run 'npm uninstall -g @rivolink/leaf'?"; then
            npm uninstall -g @rivolink/leaf
        fi
        exit 0
        ;;
esac

if ! confirm "Remove binary $BIN?"; then
    echo "Binary removal cancelled."
    exit 0
fi

rm -f "$BIN"

if [ -e "$BIN" ]; then
    err "Failed to remove $BIN."
    err "Check permissions (try with sudo)."
    exit 1
fi

echo "leaf binary removed: $BIN"
