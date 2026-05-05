#!/usr/bin/env sh
set -eu

REPO="RivoLink/leaf"
DEST_DIR="${1:-$HOME/.local/bin}"
DEST_BIN="$DEST_DIR/leaf"

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        exit 1
    fi
}

latest_tag() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSIL "https://github.com/$REPO/releases/latest" |
            sed -n 's/^[Ll]ocation: .*\/releases\/tag\/\([^[:space:]\r]*\).*/\1/p' |
            tail -n 1
    elif command -v wget >/dev/null 2>&1; then
        wget -S --max-redirect=0 -O /dev/null "https://github.com/$REPO/releases/latest" 2>&1 |
            sed -n 's/^  Location: .*\/releases\/tag\/\([^[:space:]\r]*\).*/\1/p' |
            tail -n 1
    else
        echo "Missing required command: curl or wget" >&2
        exit 1
    fi
}

download_to() {
    url="$1"
    output="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$output"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$output" "$url"
    else
        echo "Missing required command: curl or wget" >&2
        exit 1
    fi
}

detect_asset() {
    os_name="$(uname -s)"
    arch_name="$(uname -m)"
    os_extra="$(uname -o 2>/dev/null || true)"

    case "$os_name:$arch_name:$os_extra" in
        Linux:aarch64:Android | Linux:arm64:Android)
            echo "leaf-android-arm64"
            return 0
            ;;
    esac

    if [ "${TERMUX_VERSION:-}" != "" ]; then
        case "$arch_name" in
            aarch64 | arm64)
                echo "leaf-android-arm64"
                return 0
                ;;
        esac
    fi

    case "$os_name" in
        Darwin)
            case "$arch_name" in
                x86_64 | amd64)
                    echo "leaf-macos-x86_64"
                    ;;
                arm64 | aarch64)
                    echo "leaf-macos-arm64"
                    ;;
                *)
                    echo "Unsupported macOS architecture: $arch_name" >&2
                    exit 1
                    ;;
            esac
            ;;
        Linux)
            case "$arch_name" in
                x86_64 | amd64)
                    echo "leaf-linux-x86_64"
                    ;;
                aarch64 | arm64)
                    echo "leaf-linux-arm64"
                    ;;
                *)
                    echo "Unsupported Linux architecture: $arch_name" >&2
                    exit 1
                    ;;
            esac
            ;;
        *)
            echo "Unsupported platform: $os_name $arch_name" >&2
            exit 1
            ;;
    esac
}

need_cmd sed
need_cmd uname
need_cmd mktemp
need_cmd chmod
need_cmd mkdir
need_cmd cp

asset_name="$(detect_asset)"
tag_name="$(latest_tag)"

if [ -z "$tag_name" ]; then
    echo "Unable to resolve latest release tag for $REPO" >&2
    exit 1
fi

base_url="https://github.com/$REPO/releases/download/$tag_name"
tmp_file="$(mktemp)"
tmp_checksums="$(mktemp)"
trap 'rm -f "$tmp_file" "$tmp_checksums"' EXIT

mkdir -p "$DEST_DIR"
download_to "$base_url/$asset_name" "$tmp_file"
download_to "$base_url/checksums.txt" "$tmp_checksums"

expected="$(grep "[[:space:]]${asset_name}$" "$tmp_checksums" | awk '{print $1}')"
if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$tmp_file" | awk '{print $1}')"
else
    actual="$(shasum -a 256 "$tmp_file" | awk '{print $1}')"
fi
[ "$actual" = "$expected" ] || { echo "Checksum mismatch for $asset_name" >&2; exit 1; }

cp "$tmp_file" "$DEST_BIN"
chmod 755 "$DEST_BIN"

echo "Installed leaf $tag_name to $DEST_BIN"
case ":$PATH:" in
    *:"$DEST_DIR":*)
        ;;
    *)
        echo "Add $DEST_DIR to PATH if needed."
        ;;
esac
