#!/bin/sh
set -e

REPO="rigogsilva/pensieve"
INSTALL_DIR="$HOME/bin"

# Detect platform
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
  Darwin) PLATFORM="apple-darwin" ;;
  Linux)  PLATFORM="unknown-linux-gnu" ;;
  *)      echo "Unsupported OS: $OS" && exit 1 ;;
esac

case "$ARCH" in
  arm64|aarch64) ARCH="aarch64" ;;
  x86_64)        ARCH="x86_64" ;;
  *)             echo "Unsupported architecture: $ARCH" && exit 1 ;;
esac

BINARY="pensieve-${ARCH}-${PLATFORM}"
URL="https://github.com/${REPO}/releases/latest/download/${BINARY}"

echo "Installing pensieve (${ARCH}-${PLATFORM})..."

mkdir -p "$INSTALL_DIR"
curl -fsSL "$URL" -o "$INSTALL_DIR/pensieve"
chmod +x "$INSTALL_DIR/pensieve"

# Re-sign on macOS (downloaded binaries lose their ad-hoc signature on Apple Silicon)
if [ "$OS" = "Darwin" ] && command -v codesign >/dev/null 2>&1; then
  codesign --force --sign - "$INSTALL_DIR/pensieve" 2>/dev/null || true
fi

echo "Installed to $INSTALL_DIR/pensieve"
echo ""

# Run setup
"$INSTALL_DIR/pensieve" setup
