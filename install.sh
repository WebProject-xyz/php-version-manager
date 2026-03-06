#!/usr/bin/env bash
set -e

# PVM Auto-Install Script
# Downloads the latest precompiled release binary from GitHub.

GITHUB_REPO="WebProject-xyz/php-version-manager"
PVM_DIR="${PVM_DIR:-$HOME/.local/share/pvm}"
PVM_BIN_DIR="$PVM_DIR/bin"

echo "Installing PHP Version Manager (pvm)..."

# 1. Detect OS
OS="$(uname -s)"
case "$OS" in
    Linux)
        TARGET_OS="linux"
        ;;
    Darwin)
        TARGET_OS="macos"
        ;;
    *)
        echo "Error: Unsupported OS '$OS'"
        exit 1
        ;;
esac

# 2. Detect Architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)
        TARGET_ARCH="x86_64"
        ;;
    arm64|aarch64)
        TARGET_ARCH="aarch64"
        ;;
    *)
        echo "Error: Unsupported architecture '$ARCH'"
        exit 1
        ;;
esac

ASSET_NAME="pvm-${TARGET_OS}-${TARGET_ARCH}.tar.gz"

echo "Detected Platform: $TARGET_OS ($TARGET_ARCH)"

# 3. Fetch latest release version from GitHub API
echo "Fetching latest release from GitHub..."
LATEST_RELEASE_URL="https://api.github.com/repos/$GITHUB_REPO/releases/latest"

# Simple curl to get the latest tag name (e.g., "v0.1.0")
if ! command -v curl &> /dev/null; then
    echo "Error: curl is required to download pvm."
    exit 1
fi

LATEST_TAG=$(curl -sSL "$LATEST_RELEASE_URL" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
    echo "Error: Could not determine the latest release version. Maybe rate-limited?"
    exit 1
fi

echo "Latest Version: $LATEST_TAG"

# 4. Construct download URL
DOWNLOAD_URL="https://github.com/$GITHUB_REPO/releases/download/$LATEST_TAG/$ASSET_NAME"
TEMP_ARCHIVE="/tmp/pvm-installer.tar.gz"

echo "Downloading $DOWNLOAD_URL ..."
curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_ARCHIVE"

# 5. Extract and Install
echo "Extracting binary..."
mkdir -p "$PVM_BIN_DIR"

# Change into PVM_BIN_DIR and extract only the pvm executable from the tarball
tar -xzf "$TEMP_ARCHIVE" -C "$PVM_BIN_DIR" pvm

chmod +x "$PVM_BIN_DIR/pvm"
rm -f "$TEMP_ARCHIVE"

echo "Successfully installed pvm $LATEST_TAG to $PVM_BIN_DIR/pvm!"
echo ""

# 6. Automatic Profile Configuration
PROFILE=""
if [ -n "$BASH_VERSION" ]; then
    PROFILE="$HOME/.bashrc"
elif [ -n "$ZSH_VERSION" ]; then
    PROFILE="$HOME/.zshrc"
elif [ -f "$HOME/.config/fish/config.fish" ]; then
    PROFILE="$HOME/.config/fish/config.fish"
elif [ -f "$HOME/.bash_profile" ]; then
    PROFILE="$HOME/.bash_profile"
elif [ -f "$HOME/.profile" ]; then
    PROFILE="$HOME/.profile"
fi

if [ -n "$PROFILE" ] && [ -f "$PROFILE" ]; then
    HOOK_STRING="eval \"\$($PVM_BIN_DIR/pvm env)\""
    
    if ! grep -q "pvm env" "$PROFILE"; then
        echo "" >> "$PROFILE"
        echo "# PHP Version Manager (pvm)" >> "$PROFILE"
        echo "$HOOK_STRING" >> "$PROFILE"
        echo "✅ Automatically added pvm to your $PROFILE profile."
    else
        echo "✅ pvm is already configured in your $PROFILE profile."
    fi
else
    echo "⚠️ Could not automatically detect your shell profile."
    echo "Please add the following to your shell profile (.bashrc, .zshrc, or .config/fish/config.fish):"
    echo "  eval \"\$($PVM_BIN_DIR/pvm env)\""
fi

echo ""
echo "Please restart your shell or run 'source $PROFILE' to start using pvm!"
