#!/bin/bash
set -e

echo "=== Shelfy Linux Build Script ==="
echo "This will install all dependencies and build the AppImage."
echo ""

# Install system dependencies
echo "[1/5] Installing system dependencies..."
sudo apt-get update -qq
sudo apt-get install -y -qq \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  pkg-config \
  git \
  fuse \
  libfuse2

# Install Node.js 22
echo "[2/5] Installing Node.js 22..."
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo bash - >/dev/null 2>&1
sudo apt-get install -y -qq nodejs

# Install Rust
echo "[3/5] Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y >/dev/null 2>&1
source "$HOME/.cargo/env"

# Clone repo if not present
if [ ! -d "$HOME/shelfy" ]; then
  echo "[4/5] Cloning Shelfy repository..."
  git clone https://github.com/hsr88/shelfy.git "$HOME/shelfy"
fi

cd "$HOME/shelfy"

# Build
echo "[5/5] Building AppImage (this takes ~5-10 minutes)..."
npm install
npm run tauri build

echo ""
echo "=== BUILD COMPLETE ==="
echo "Your files are in: $HOME/shelfy/src-tauri/target/release/bundle/"
echo ""
echo "AppImage:  $HOME/shelfy/src-tauri/target/release/bundle/appimage/Shelfy_0.1.2_amd64.AppImage"
echo ".deb:      $HOME/shelfy/src-tauri/target/release/bundle/deb/Shelfy_0.1.2_amd64.deb"
echo ".rpm:      $HOME/shelfy/src-tauri/target/release/bundle/rpm/Shelfy-0.1.2-1.x86_64.rpm"
echo ""
echo "To copy AppImage back to Windows, use Shared Folder or drag & drop."
