#!/bin/bash
set -u

# FREECODE Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/mr-kelly/freecode/main/install.sh | bash

abort() {
  printf "%s\n" "$@" >&2
  exit 1
}

if [ -z "${BASH_VERSION:-}" ]; then
  abort "Bash is required to interpret this script."
fi

if [[ -n "${POSIXLY_CORRECT+1}" ]]; then
  abort 'Bash must not run in POSIX mode. Please unset POSIXLY_CORRECT and try again.'
fi

REPO="${FREECODE_REPO:-mr-kelly/freecode}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)  OS_TYPE="linux" ;;
    Darwin*) OS_TYPE="macos" ;;
    *)       abort "Unsupported OS: $OS" ;;
esac

case "$ARCH" in
    x86_64|amd64)    ARCH_TYPE="x86_64" ;;
    aarch64|arm64)   ARCH_TYPE="aarch64" ;;
    *)               abort "Unsupported architecture: $ARCH" ;;
esac

BINARY_NAME="freecode-${OS_TYPE}-${ARCH_TYPE}"
TARGET_BIN="${INSTALL_DIR}/freecode"

echo "Installing freecode for ${OS_TYPE}-${ARCH_TYPE}..."

EXISTING="$(command -v freecode 2>/dev/null || true)"
if [ -n "$EXISTING" ] && [ "$EXISTING" != "$TARGET_BIN" ]; then
    echo "⚠️  Existing 'freecode' found at: $EXISTING"
    echo "    This installer will place freecode at: $TARGET_BIN"
    echo ""
fi

RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>&1)
if echo "$RELEASE_JSON" | grep -q "rate limit"; then
  abort "GitHub API rate limit exceeded. Please try again later or set GITHUB_TOKEN."
fi

LATEST_RELEASE=$(printf '%s' "$RELEASE_JSON" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || true)

if [ -z "$LATEST_RELEASE" ]; then
    abort "Failed to get latest release for ${REPO}. Check if releases exist."
fi

echo "Latest version: $LATEST_RELEASE"

DOWNLOAD_URL=$(
    printf '%s' "$RELEASE_JSON" \
    | grep '"browser_download_url":' \
    | sed -E 's/.*"([^"]+)".*/\1/' \
    | grep -E "/${BINARY_NAME}(-[^/]+)?\\.tar\\.gz$" \
    | head -n1
)

if [ -z "$DOWNLOAD_URL" ]; then
    echo "No matching binary asset found for ${BINARY_NAME} in release ${LATEST_RELEASE}"
    echo "Available assets:"
    printf '%s' "$RELEASE_JSON" \
      | grep '"name":' \
      | sed -E 's/.*"name": "([^"]+)".*/  - \1/' \
      || true
    abort "Installation failed."
fi

echo "Downloading from: $DOWNLOAD_URL"

TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

cd "$TMP_DIR"
curl -fsSL "$DOWNLOAD_URL" -o freecode.tar.gz
tar xzf freecode.tar.gz

mkdir -p "$INSTALL_DIR"
mv freecode "$TARGET_BIN"
chmod +x "$TARGET_BIN"

echo ""
echo "✅ freecode installed successfully to $TARGET_BIN"
echo ""

if echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo "You can now run: freecode \"your task\""
else
    echo "⚠️  Add $INSTALL_DIR to your PATH:"
    echo ""
    SHELL_NAME=$(basename "$SHELL" 2>/dev/null || echo "bash")
    case "$SHELL_NAME" in
        zsh)  RC_FILE="$HOME/.zshrc" ;;
        bash) RC_FILE="$HOME/.bashrc" ;;
        fish) RC_FILE="$HOME/.config/fish/config.fish" ;;
        *)    RC_FILE="$HOME/.profile" ;;
    esac

    echo "    echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> $RC_FILE"
    echo "    source $RC_FILE"
    echo ""
    echo "Or run directly: $TARGET_BIN"
fi

ACTIVE="$(command -v freecode 2>/dev/null || true)"
if [ -n "$ACTIVE" ] && [ "$ACTIVE" != "$TARGET_BIN" ]; then
    echo ""
    echo "⚠️  PATH priority notice:"
    echo "    'freecode' currently resolves to: $ACTIVE"
    echo "    Newly installed binary is at: $TARGET_BIN"
    echo "    Run 'which -a freecode' and adjust PATH order if needed."
fi

echo ""
echo "📚 Documentation: https://github.com/${REPO}"
echo "🐛 Report issues: https://github.com/${REPO}/issues"
