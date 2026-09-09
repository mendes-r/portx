#!/usr/bin/env bash
set -euo pipefail

prog="portx"
install_dir="${INSTALL_DIR:-/usr/local/bin}"

os="$(uname -s)"
case "$os" in
    Darwin|Linux) ;;
    *)
        echo "error: unsupported OS: $os (this script supports macOS and Linux only)" >&2
        exit 1
        ;;
esac

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found. Install Rust first: https://rustup.rs" >&2
    exit 1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir"

echo "Building $prog (release)..."
cargo build --release

binary="target/release/$prog"
if [ ! -f "$binary" ]; then
    echo "error: build succeeded but binary not found at $binary" >&2
    exit 1
fi

if [ -w "$install_dir" ]; then
    install -m 755 "$binary" "$install_dir/$prog"
else
    echo "Installing to $install_dir requires sudo..."
    sudo install -m 755 "$binary" "$install_dir/$prog"
fi

echo "Installed $prog to $install_dir/$prog"
