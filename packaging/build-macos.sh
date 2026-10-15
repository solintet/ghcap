#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
RELEASE_VERSION="0.0.29.8"
CARGO_VERSION="0.29.8"
DIST="$ROOT/dist"

usage() {
  echo "Usage: $0 intel|apple-silicon" >&2
  exit 2
}
[ "$#" -eq 1 ] || usage
case "$1" in
  intel)
    TARGET="x86_64-apple-darwin"
    ARCHIVE_ARCH="intel"
    EXPECTED_UNAME="x86_64"
    export MACOSX_DEPLOYMENT_TARGET="10.13"
    ;;
  apple-silicon)
    TARGET="aarch64-apple-darwin"
    ARCHIVE_ARCH="apple-silicon"
    EXPECTED_UNAME="arm64"
    export MACOSX_DEPLOYMENT_TARGET="11.0"
    ;;
  *) usage ;;
esac

for command in cargo rustc rustup shasum tar uname; do
  command -v "$command" >/dev/null 2>&1 || { echo "ERROR: Required command '$command' is missing." >&2; exit 1; }
done
if [ "$(uname -s)" != "Darwin" ]; then
  echo "ERROR: This script must run on a macOS host or GitHub-hosted macOS runner." >&2
  exit 1
fi
HOST_ARCH=$(uname -m)
if [ "$HOST_ARCH" != "$EXPECTED_UNAME" ]; then
  echo "ERROR: Requested $ARCHIVE_ARCH build, but host architecture is $HOST_ARCH (expected $EXPECTED_UNAME)." >&2
  exit 1
fi
grep -q "^version = \"$CARGO_VERSION\"$" Cargo.toml || { echo "ERROR: Cargo.toml version must be $CARGO_VERSION." >&2; exit 1; }

if [ ! -f Cargo.lock ]; then
  echo "Cargo.lock is missing; resolving dependencies (network required on first run)."
  cargo generate-lockfile
fi
rustup target add "$TARGET"
mkdir -p "$DIST"
cargo fmt --all -- --check
cargo check --all-targets --all-features --locked
cargo test --all-targets --all-features --locked
cargo build --release --locked --target "$TARGET"

BINARY="$ROOT/target/$TARGET/release/ghcap"
[ -s "$BINARY" ] || { echo "ERROR: Expected binary not found: $BINARY" >&2; exit 1; }
STAGE="$ROOT/.packaging/macos-$ARCHIVE_ARCH"
rm -rf "$STAGE"
mkdir -p "$STAGE"
install -m 0755 "$BINARY" "$STAGE/ghcap"
cp README.md LICENSE DEPENDENCIES.md "$STAGE/"
cat > "$STAGE/INSTALL.txt" <<EOF2
ghcap macOS $ARCHIVE_ARCH build
Requires Git, GitHub CLI (gh), and GnuPG (gpg).
Install dependencies with Homebrew if available: brew install git gh gnupg
Run ./ghcap from Terminal.
This binary is not code-signed or notarized; macOS may show a Gatekeeper warning.
EOF2
ARCHIVE="$DIST/ghcap-v$RELEASE_VERSION-macos-$ARCHIVE_ARCH.tar.gz"
tar -C "$STAGE" -czf "$ARCHIVE" ghcap README.md LICENSE DEPENDENCIES.md INSTALL.txt
(cd "$DIST" && shasum -a 256 "$(basename "$ARCHIVE")" > "$(basename "$ARCHIVE").sha256")
(cd "$DIST" && shasum -a 256 "$(basename "$ARCHIVE")" > SHA256SUMS)
echo "macOS $ARCHIVE_ARCH archive: $ARCHIVE"
echo "SHA-256: $(awk '{print $1}' "$ARCHIVE.sha256")"
