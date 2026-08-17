#!/bin/bash

# Configuration
GITHUB_REPO="bircoder432/giss"

# Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux*)  TARGET_OS="unknown-linux-gnu" ;;
  Darwin*) TARGET_OS="apple-darwin" ;;
  MINGW*|MSYS*|CYGWIN*) TARGET_OS="pc-windows-msvc" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) TARGET_ARCH="x86_64" ;;
  aarch64|arm64) TARGET_ARCH="aarch64" ;;
  *) echo "Unsupported Architecture: $ARCH"; exit 1 ;;
esac

TARGET_TRIPLE="${TARGET_ARCH}-${TARGET_OS}"

if [ "$TARGET_OS" == "pc-windows-msvc" ]; then
  ARCHIVE_EXT="zip"
else
  ARCHIVE_EXT="tar.gz"
fi

# Fetch latest release tag
echo "Fetching latest release..."
LATEST_TAG=$(curl -s "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
  echo "Failed to fetch latest release. Check your internet connection or repo name."
  exit 1
fi

echo "Latest release is $LATEST_TAG"

ASSET_NAME="giss-${TARGET_TRIPLE}.${ARCHIVE_EXT}"
DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${LATEST_TAG}/${ASSET_NAME}"

echo "Downloading $ASSET_NAME..."
TEMP_DIR=$(mktemp -d)
curl -sL "$DOWNLOAD_URL" -o "$TEMP_DIR/$ASSET_NAME"

if [ $? -ne 0 ]; then
  echo "Download failed."
  exit 1
fi

echo "Extracting..."
EXTRACT_DIR="$TEMP_DIR/extract"
mkdir -p "$EXTRACT_DIR"

if [ "$ARCHIVE_EXT" == "zip" ]; then
  unzip -q "$TEMP_DIR/$ASSET_NAME" -d "$EXTRACT_DIR"
else
  tar -xzf "$TEMP_DIR/$ASSET_NAME" -C "$EXTRACT_DIR"
fi

# Interactive selection
echo ""
echo "Select components to install:"
echo "1) giss (User CLI)"
echo "2) gism (Server Manager)"
echo "3) gistui (TUI Client)"
echo "4) All"
echo ""
read -p "Enter numbers separated by space (e.g., 1 3): " choices

INSTALL_DIR="/usr/local/bin"
SUDO=""

# Check if we need sudo
if [ ! -w "$INSTALL_DIR" ]; then
  SUDO="sudo"
fi

INSTALL_GISS=false
INSTALL_GISM=false
INSTALL_GISTUI=false

for choice in $choices; do
  case $choice in
    1) INSTALL_GISS=true ;;
    2) INSTALL_GISM=true ;;
    3) INSTALL_GISTUI=true ;;
    4) INSTALL_GISS=true; INSTALL_GISM=true; INSTALL_GISTUI=true ;;
    *) echo "Invalid choice: $choice" ;;
  esac
done

# Install function
install_bin() {
  local bin_name=$1
  if [ "$TARGET_OS" == "pc-windows-msvc" ]; then
    bin_name="${bin_name}.exe"
  fi

  if [ -f "$EXTRACT_DIR/$bin_name" ]; then
    echo "Installing $bin_name to $INSTALL_DIR..."
    $SUDO cp "$EXTRACT_DIR/$bin_name" "$INSTALL_DIR/"
    $SUDO chmod +x "$INSTALL_DIR/$bin_name"
    echo "$bin_name installed successfully."
  else
    echo "Binary $bin_name not found in the archive."
  fi
}

if [ "$INSTALL_GISS" = true ]; then install_bin "giss"; fi
if [ "$INSTALL_GISM" = true ]; then install_bin "gism"; fi
if [ "$INSTALL_GISTUI" = true ]; then install_bin "gistui"; fi

# Cleanup
rm -rf "$TEMP_DIR"

echo ""
echo "Installation complete."
if [ "$INSTALL_GISS" = true ] || [ "$INSTALL_GISTUI" = true ]; then
  echo "Note: Ensure $INSTALL_DIR is in your PATH."
  echo "For gistui and giss, create a config file at ~/.config/giss/config.toml"
fi
