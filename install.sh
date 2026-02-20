#!/usr/bin/env bash
set -euo pipefail

REPO="marcelsud/shipit"
INSTALL_DIR="${SHIPIT_INSTALL_DIR:-$HOME/.local/bin}"

info() { printf '\033[1;34m%s\033[0m\n' "$*"; }
error() { printf '\033[1;31merror: %s\033[0m\n' "$*" >&2; exit 1; }

# Detect OS
case "$(uname -s)" in
  Linux*)  OS="linux" ;;
  Darwin*) OS="macos" ;;
  *)       error "Unsupported OS: $(uname -s)" ;;
esac

# Detect architecture
case "$(uname -m)" in
  x86_64|amd64)  ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *)             error "Unsupported architecture: $(uname -m)" ;;
esac

# Map to asset name
if [ "$OS" = "linux" ] && [ "$ARCH" = "x86_64" ]; then
  ASSET="shipit-x86_64-linux.tar.gz"
elif [ "$OS" = "macos" ] && [ "$ARCH" = "aarch64" ]; then
  ASSET="shipit-aarch64-macos.tar.gz"
elif [ "$OS" = "macos" ] && [ "$ARCH" = "x86_64" ]; then
  ASSET="shipit-x86_64-macos.tar.gz"
else
  error "No pre-built binary for $OS/$ARCH"
fi

# Get latest release download URL
info "Fetching latest release..."
DOWNLOAD_URL=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep -o "https://github.com/$REPO/releases/download/[^\"]*/$ASSET" \
  | head -1)

if [ -z "$DOWNLOAD_URL" ]; then
  error "Could not find release asset: $ASSET"
fi

# Create install directory
mkdir -p "$INSTALL_DIR"

# Download and extract
info "Downloading $ASSET..."
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL "$DOWNLOAD_URL" -o "$TMPDIR/$ASSET"
tar xzf "$TMPDIR/$ASSET" -C "$TMPDIR"
mv "$TMPDIR/shipit" "$INSTALL_DIR/shipit"
chmod +x "$INSTALL_DIR/shipit"

info "Installed shipit to $INSTALL_DIR/shipit"

# Check PATH
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    printf '\033[1;33m%s\033[0m\n' "Warning: $INSTALL_DIR is not in your PATH."
    echo "Add this to your shell profile:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac

info "Done! Run 'shipit --version' to verify."
