#!/usr/bin/env bash
#
# Install the latest (or a specified) release of srv from GitHub.
# Supports Linux arm64 (Raspberry Pi 5, Raspberry Pi OS Lite / Debian Trixie).
# Usage: ./inst.sh [VERSION]      e.g. ./inst.sh v0.1.0
#        INSTALL_DIR=$HOME/.local/bin ./inst.sh
# Piped: curl --proto '=https' --tlsv1.2 -sSfL <url>/inst.sh | bash -s -- v0.1.0

set -Eeuo pipefail

REPO="ckunte/srv"
BINARY="srv"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
CHECKSUM_FILE="SHA256SUMS.txt"

log()  { printf '%s\n' "$*"; }
fail() { printf 'Error: %s\n' "$1" >&2; exit "${2:-1}"; }

# --- Preconditions ---------------------------------------------------------
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS/$ARCH" in
    Linux/aarch64|Linux/arm64)   PLATFORM="linux-arm64-rpi5" ;;
    *) fail "Unsupported system: $OS/$ARCH (prebuilt binaries are for Linux arm64 only; see the README to build from source)." ;;
esac

if command -v sha256sum >/dev/null 2>&1; then
    SHA_CMD="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
    SHA_CMD="shasum -a 256"
else
    fail "Required command not found: sha256sum or shasum"
fi

for cmd in curl tar install awk mktemp; do
    command -v "$cmd" >/dev/null 2>&1 || fail "Required command not found: $cmd"
done

# --- Work in a private temp dir, always cleaned up -------------------------
WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/srv-inst.XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
trap 'fail "Interrupted." 130' INT TERM
trap 'fail "Unexpected failure on line $LINENO."' ERR

# --- 1. Resolve version ----------------------------------------------------
VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    printf 'Checking the latest release version for %s... ' "$REPO"
    API_URL="https://api.github.com/repos/${REPO}/releases/latest"
    RESPONSE="$(curl -fsSL --retry 3 --connect-timeout 10 --max-time 30 \
        -H 'Accept: application/vnd.github+json' "$API_URL")" \
        || { printf 'failed.\n'; fail "Could not reach the GitHub API (rate limit or network?)."; }

    if command -v jq >/dev/null 2>&1; then
        VERSION="$(printf '%s' "$RESPONSE" | jq -r '.tag_name // empty')"
    else
        VERSION="$(printf '%s' "$RESPONSE" \
            | sed -nE 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' | head -n1)"
    fi
    [ -n "$VERSION" ] || { printf 'failed.\n'; fail "Could not parse the latest release tag."; }
    printf 'done. (Found version: %s)\n' "$VERSION"
fi

# Guard against odd characters ending up in URLs/filenames
case "$VERSION" in
    *[!A-Za-z0-9._-]*|"") fail "Unexpected version string: '$VERSION'" ;;
esac

ARCHIVE="${BINARY}-${VERSION}-${PLATFORM}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"

# --- 2. Download -----------------------------------------------------------
download() {
    local name="$1"
    printf 'Downloading %s... ' "$name"
    if curl -fsSL --retry 3 --connect-timeout 10 --max-time 300 \
        -o "$WORKDIR/$name" "$BASE_URL/$name"; then
        printf 'done.\n'
    else
        printf 'failed.\n'
        fail "Download failed: $BASE_URL/$name"
    fi
}
download "$ARCHIVE"
download "$CHECKSUM_FILE"

# --- 3. Verify checksum ----------------------------------------------------
printf 'Verifying SHA256 checksum... '
# Exact filename match (handles "hash  name" and "hash *name"), exactly one entry.
MATCHES="$(awk -v f="$ARCHIVE" '{n=$2; sub(/^\*/,"",n); if (n==f) print}' "$WORKDIR/$CHECKSUM_FILE")"
if [ -z "$MATCHES" ] || [ "$(printf '%s\n' "$MATCHES" | wc -l | tr -d ' ')" -ne 1 ]; then
    printf 'failed.\n'
    fail "Expected exactly one checksum entry for $ARCHIVE in $CHECKSUM_FILE." 2
fi
if (cd "$WORKDIR" && printf '%s\n' "$MATCHES" | $SHA_CMD -c - >/dev/null 2>&1); then
    printf 'done. (PASSED)\n'
else
    printf 'failed.\n'
    fail "Checksum verification FAILED. Nothing was installed." 2
fi

# --- 4. Extract (only the binary, and only if it's a plain top-level file) -
printf 'Extracting archive... '
MEMBER="$(tar -tzf "$WORKDIR/$ARCHIVE" \
    | awk -v b="$BINARY" '$0==b || $0=="./"b { if (!m) m=$0 } END { if (m) print m }')"
[ -n "$MEMBER" ] || { printf 'failed.\n'; fail "Executable '$BINARY' not found at the top level of the archive."; }
tar -xzf "$WORKDIR/$ARCHIVE" -C "$WORKDIR" "$MEMBER"
printf 'done.\n'
[ -f "$WORKDIR/$BINARY" ] || fail "Extraction did not produce '$BINARY'."
chmod 755 "$WORKDIR/$BINARY"

# --- 5. Install (sudo only when needed) ------------------------------------
printf "Installing '%s' to %s... " "$BINARY" "$INSTALL_DIR"
SUDO=""
if [ -d "$INSTALL_DIR" ]; then
    [ -w "$INSTALL_DIR" ] || SUDO="sudo"
else
    mkdir -p "$INSTALL_DIR" 2>/dev/null || SUDO="sudo"
    [ -z "$SUDO" ] || $SUDO mkdir -p "$INSTALL_DIR"
fi
if [ -n "$SUDO" ]; then
    command -v sudo >/dev/null 2>&1 || fail "sudo is required to write to $INSTALL_DIR."
    printf '\n(sudo required) '
fi
$SUDO install -m 755 "$WORKDIR/$BINARY" "$INSTALL_DIR/$BINARY"
printf 'done.\n'

log "Successfully installed $BINARY $VERSION ($PLATFORM) to $INSTALL_DIR/$BINARY"

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) log "Note: $INSTALL_DIR is not on your PATH." ;;
esac

"$INSTALL_DIR/$BINARY" help || true
