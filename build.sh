#!/usr/bin/env bash
# Bauer Browser — Build & Run (Linux / macOS)
set -euo pipefail

echo ""
echo "╔══════════════════════════════════╗"
echo "║   Bauer Browser — Linux/macOS    ║"
echo "╚══════════════════════════════════╝"
echo ""

# ── Rust ──────────────────────────────────────────────────────────────────────
if ! command -v cargo &>/dev/null; then
    echo "✗ Rust/Cargo not found."
    echo "  Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi
echo "✔ $(cargo --version)"

# ── Platform dependencies ─────────────────────────────────────────────────────
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "Checking Linux WebKitGTK dependencies..."
    if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null && \
       ! pkg-config --exists webkit2gtk-4.0 2>/dev/null; then
        echo ""
        echo "✗ WebKitGTK not found. Install:"
        echo "  Ubuntu 24.04: sudo apt install libwebkitgtk-6.0-dev libgtk-3-dev"
        echo "  Ubuntu 22.04: sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev"
        echo "  Fedora:       sudo dnf install webkit2gtk4.1-devel gtk3-devel"
        exit 1
    fi
    echo "✔ WebKitGTK found"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    echo "✔ macOS: WKWebView is built-in"
fi

# ── Config ────────────────────────────────────────────────────────────────────
if [ ! -f config/settings.toml ]; then
    cp config/defaults.toml config/settings.toml
    echo "✔ Created config/settings.toml"
fi

# ── Build ─────────────────────────────────────────────────────────────────────
echo ""
MODE="${1:-debug}"
if [ "$MODE" = "release" ]; then
    echo "Building release (optimized)..."
    cargo build --release
    BIN="./target/release/bauer-browser"
else
    echo "Building debug..."
    cargo build
    BIN="./target/debug/bauer-browser"
fi

echo ""
echo "✔ Build successful: $BIN"
echo ""

if [ "${2:-}" = "run" ] || [ "$MODE" = "run" ]; then
    echo "Starting Bauer Browser..."
    exec "$BIN"
fi
