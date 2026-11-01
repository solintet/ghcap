#!/bin/sh
set -eu
say() { printf '%s\n' "$*"; }
fail() { say "ERROR: $*" >&2; exit 1; }
command -v xcode-select >/dev/null 2>&1 || fail "xcode-select is missing. This script must run on macOS."
if ! xcode-select -p >/dev/null 2>&1; then
  say "Xcode Command Line Tools are not installed. Starting Apple's installer..."
  xcode-select --install || true
  say "Finish the graphical installer, then run ./packaging/setup-macos.sh again."
  exit 0
fi
command -v xcrun >/dev/null 2>&1 || fail "xcrun is unavailable; repair Command Line Tools."
xcrun --find clang >/dev/null 2>&1 || fail "Apple clang is unavailable; repair Command Line Tools."
if ! command -v rustup >/dev/null 2>&1; then
  command -v curl >/dev/null 2>&1 || fail "curl is required to install Rust from the official rustup site."
  tmp=$(mktemp)
  trap 'rm -f "$tmp"' EXIT HUP INT TERM
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$tmp"
  sh "$tmp" -y --profile minimal --default-toolchain stable
  rm -f "$tmp"
  trap - EXIT HUP INT TERM
  export PATH="$HOME/.cargo/bin:$PATH"
fi
command -v cargo >/dev/null 2>&1 || fail "cargo is not on PATH; open a new Terminal or run: . \"$HOME/.cargo/env\""
rustup toolchain install stable
rustup default stable
case "$(uname -m)" in
  x86_64) TARGET=x86_64-apple-darwin ;;
  arm64) TARGET=aarch64-apple-darwin ;;
  *) fail "Unsupported Mac host architecture: $(uname -m)" ;;
esac
rustup target add "$TARGET"
say "Setup complete for native architecture: $TARGET"
say "macOS SDK: $(xcrun --sdk macosx --show-sdk-path)"
say "clang: $(xcrun --find clang)"
say "rustc: $(rustc --version)"
say "cargo: $(cargo --version)"
