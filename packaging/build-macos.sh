#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
RELEASE_VERSION="0.0.29.0"
CARGO_VERSION="0.29.0"
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

for command in cargo rustc rustup shasum tar uname file git; do
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
  echo "ERROR: A reviewed Cargo.lock is required." >&2
  exit 1
fi
rustup target add "$TARGET"
mkdir -p "$DIST"
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo check --all-targets --all-features --locked
cargo test --all-targets --all-features --locked
cargo build --release --locked --target "$TARGET"

BINARY="$ROOT/target/$TARGET/release/ghcap"
[ -s "$BINARY" ] || { echo "ERROR: Expected binary not found: $BINARY" >&2; exit 1; }
STAGE=$(mktemp -d "${TMPDIR:-/tmp}/ghcap-package.XXXXXXXX")
trap 'rm -rf "$STAGE"' EXIT HUP INT TERM
install -m 0755 "$BINARY" "$STAGE/ghcap"
cp README.md LICENSE DEPENDENCIES.md THIRD_PARTY_NOTICES.md "$STAGE/"
cp -R THIRD_PARTY_LICENSES "$STAGE/"
file "$BINARY" | grep -q "$(case "$ARCHIVE_ARCH" in intel) echo x86_64;; *) echo arm64;; esac)" || { echo "ERROR: Binary architecture mismatch." >&2; exit 1; }
"$BINARY" --version | grep -qx "ghcap $RELEASE_VERSION"
git rev-parse HEAD > "$STAGE/SOURCE_COMMIT.txt"
git diff --quiet HEAD -- || { echo "ERROR: Dirty source cannot be packaged." >&2; exit 1; }
{ rustc -Vv; cargo -V; shasum -a 256 Cargo.lock; } > "$STAGE/BUILD_INFO.txt"
cat > "$STAGE/INSTALL.txt" <<EOF2
ghcap macOS $ARCHIVE_ARCH build
Requires Git, GitHub CLI (gh), and GnuPG (gpg).
Install dependencies with Homebrew if available: brew install git gh gnupg
Run ./ghcap from Terminal.
This binary is not code-signed or notarized; macOS may show a Gatekeeper warning.
EOF2
ARCHIVE="$DIST/ghcap-v$RELEASE_VERSION-macos-$ARCHIVE_ARCH.tar.gz"
[ ! -e "$ARCHIVE" ] || { echo "ERROR: Refusing to overwrite $ARCHIVE" >&2; exit 1; }
tar -C "$STAGE" -czf "$ARCHIVE" ghcap README.md LICENSE DEPENDENCIES.md INSTALL.txt SOURCE_COMMIT.txt BUILD_INFO.txt THIRD_PARTY_NOTICES.md THIRD_PARTY_LICENSES
(cd "$DIST" && shasum -a 256 "$(basename "$ARCHIVE")" > "$(basename "$ARCHIVE").sha256")
(cd "$DIST" && cat ./*.tar.gz.sha256 > SHA256SUMS)
mkdir "$STAGE/verify"
tar -C "$STAGE/verify" -xzf "$ARCHIVE"
"$STAGE/verify/ghcap" --version | grep -qx "ghcap $RELEASE_VERSION"
"$STAGE/verify/ghcap" --help >/dev/null
(cd "$DIST" && shasum -a 256 -c "$(basename "$ARCHIVE").sha256")
echo "macOS $ARCHIVE_ARCH archive: $ARCHIVE"
echo "SHA-256: $(awk '{print $1}' "$ARCHIVE.sha256")"
