#!/usr/bin/env bash
set -e

echo "Installing Snap photo editor..."

# Create directories
mkdir -p ~/.local/bin
mkdir -p ~/.local/share/applications
mkdir -p ~/.local/share/icons/hicolor/scalable/apps

# Copy binary if it exists
if [ -f "target/release/snap" ]; then
    cp target/release/snap ~/.local/bin/snap
    chmod +x ~/.local/bin/snap
    echo "Installed binary to ~/.local/bin/snap"
else
    echo "Warning: target/release/snap not found. Please build with 'cargo build --release' first."
fi

# Copy desktop file
if [ -f "assets/snap.desktop" ]; then
    cp assets/snap.desktop ~/.local/share/applications/snap.desktop
    echo "Installed desktop file"
fi

# Copy icon
if [ -f "assets/snap.svg" ]; then
    cp assets/snap.svg ~/.local/share/icons/hicolor/scalable/apps/snap.svg
    echo "Installed SVG icon"
fi

# Update databases if available
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database ~/.local/share/applications || true
fi

if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor || true
fi

echo "Snap local installation complete!"
