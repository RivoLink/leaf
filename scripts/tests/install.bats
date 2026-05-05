#!/usr/bin/env bats

# Tests for scripts/install.sh
#
# Pre-change tests (regression coverage, tests 1-7) describe the original
# installer behaviour and must pass on both the original and the updated
# script.
#
# Checksum tests (tests 8-11) are specific to the SHA-256 verification added
# in the fix and cover the positive path and all negative paths.

SCRIPT="$BATS_TEST_DIRNAME/../install.sh"

# ── Mock helpers ──────────────────────────────────────────────────────────────

setup() {
    DEST_DIR="$(mktemp -d)"
    MOCK_BIN="$(mktemp -d)"

    # SHA-256 of the exact bytes the mock curl writes for binary downloads.
    # Used as the default value of MOCK_SHA256 so the positive path passes
    # checksum verification without any per-test configuration.
    FAKE_BINARY_SHA="$(printf 'FAKE_LEAF_BINARY' | sha256sum | awk '{print $1}')"

    export DEST_DIR MOCK_BIN FAKE_BINARY_SHA
    export MOCK_SHA256="$FAKE_BINARY_SHA"

    # mock uname — returns values from MOCK_OS / MOCK_ARCH if set
    cat > "$MOCK_BIN/uname" << 'EOF'
#!/bin/sh
case "$1" in
    -s) echo "${MOCK_OS:-Linux}" ;;
    -m) echo "${MOCK_ARCH:-x86_64}" ;;
    -o) echo "GNU/Linux" ;;
    *)  echo "${MOCK_OS:-Linux}" ;;
esac
EOF

    # mock curl — handles three distinct call patterns:
    #   1. -fsSIL  → latest_tag(): returns a fake Location header
    #   2. URL with checksums.txt → writes "$MOCK_SHA256  leaf-linux-x86_64"
    #   3. anything else → binary download, writes FAKE_LEAF_BINARY
    # Env vars:
    #   MOCK_EMPTY_TAG=1  → latest_tag() returns no output (empty tag)
    #   MOCK_SHA256       → hash written into checksums.txt
    #   MOCK_WRONG_ASSET=1 → checksums.txt lists a different asset name
    cat > "$MOCK_BIN/curl" << 'EOF'
#!/bin/sh

# Detect latest_tag call (-fsSIL flag)
for arg in "$@"; do
    case "$arg" in
        -fsSIL)
            [ "${MOCK_EMPTY_TAG:-0}" = "1" ] && exit 0
            printf 'location: https://github.com/RivoLink/leaf/releases/tag/v99.0.0\r\n'
            exit 0
            ;;
    esac
done

# Extract output file path from -o <file>
outfile=""
prev=""
for arg in "$@"; do
    [ "$prev" = "-o" ] && { outfile="$arg"; break; }
    prev="$arg"
done

# Detect checksums.txt download by URL
for arg in "$@"; do
    case "$arg" in
        *checksums.txt*)
            if [ "${MOCK_WRONG_ASSET:-0}" = "1" ]; then
                printf '%s  leaf-other-platform\n' "${MOCK_SHA256}" > "$outfile"
            else
                printf '%s  leaf-linux-x86_64\n' "${MOCK_SHA256}" > "$outfile"
            fi
            exit 0
            ;;
    esac
done

# Binary download
printf 'FAKE_LEAF_BINARY' > "$outfile"
EOF

    chmod +x "$MOCK_BIN/uname" "$MOCK_BIN/curl"

    export PATH="$MOCK_BIN:$PATH"
}

teardown() {
    rm -rf "$DEST_DIR" "$MOCK_BIN"
}

# ── Pre-change behaviour (regression coverage) ────────────────────────────────

@test "installs binary to specified DEST_DIR and exits 0" {
    run sh "$SCRIPT" "$DEST_DIR"
    [ "$status" -eq 0 ]
    [ -f "$DEST_DIR/leaf" ]
}

@test "installed binary has executable permissions (chmod 755)" {
    run sh "$SCRIPT" "$DEST_DIR"
    [ "$status" -eq 0 ]
    [ -x "$DEST_DIR/leaf" ]
}

@test "installs to ~/.local/bin when no argument is given" {
    HOME="$DEST_DIR" run sh "$SCRIPT"
    [ "$status" -eq 0 ]
    [ -f "$DEST_DIR/.local/bin/leaf" ]
}

@test "resolved tag appears in install output" {
    run sh "$SCRIPT" "$DEST_DIR"
    [ "$status" -eq 0 ]
    echo "$output" | grep -q "v99.0.0"
}

@test "exits non-zero when tag resolution returns empty" {
    export MOCK_EMPTY_TAG=1
    run sh "$SCRIPT" "$DEST_DIR"
    [ "$status" -ne 0 ]
}

@test "exits non-zero for unsupported OS" {
    export MOCK_OS=Windows
    run sh "$SCRIPT" "$DEST_DIR"
    [ "$status" -ne 0 ]
}

@test "exits non-zero for unsupported architecture" {
    export MOCK_ARCH=mips
    run sh "$SCRIPT" "$DEST_DIR"
    [ "$status" -ne 0 ]
}

# ── Checksum verification (new behaviour) ─────────────────────────────────────

@test "succeeds and installs binary when checksum matches" {
    # MOCK_SHA256 is already set to the correct hash in setup()
    run sh "$SCRIPT" "$DEST_DIR"
    [ "$status" -eq 0 ]
    [ -f "$DEST_DIR/leaf" ]
}

@test "exits non-zero when checksum does not match" {
    export MOCK_SHA256="0000000000000000000000000000000000000000000000000000000000000000"
    run sh "$SCRIPT" "$DEST_DIR"
    [ "$status" -ne 0 ]
}

@test "does not copy binary when checksum does not match" {
    export MOCK_SHA256="0000000000000000000000000000000000000000000000000000000000000000"
    run sh "$SCRIPT" "$DEST_DIR"
    [ ! -f "$DEST_DIR/leaf" ]
}

@test "exits non-zero when asset is absent from checksums.txt" {
    export MOCK_WRONG_ASSET=1
    run sh "$SCRIPT" "$DEST_DIR"
    [ "$status" -ne 0 ]
}
