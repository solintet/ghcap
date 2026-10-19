#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
RELEASE_VERSION="0.0.29.0"
CARGO_VERSION="0.29.0"
DIST="$ROOT/dist"

need() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "ERROR: Required command '$1' is missing." >&2
        echo "See packaging/README.md for Debian dependencies." >&2
        exit 1
    }
}

for command in cargo rustc rustup sha256sum tar zip unzip; do
    need "$command"
done

if [ "${GITHUB_REPOSITORY:-}" = "" ]; then
    echo "ERROR: Set GITHUB_REPOSITORY=OWNER/REPO so winget URLs point to the actual release." >&2
    exit 1
fi
case "$GITHUB_REPOSITORY" in
    */*) ;;
    *) echo "ERROR: GITHUB_REPOSITORY must be OWNER/REPO." >&2; exit 1 ;;
esac

if [ ! -f Cargo.lock ]; then
    echo "Cargo.lock is missing; generating it (network access may be required)..."
    cargo generate-lockfile
fi
grep -q "^version = \"$CARGO_VERSION\"$" Cargo.toml || {
    echo "ERROR: Cargo.toml version must be $CARGO_VERSION." >&2; exit 1;
}

# Linux native formats; this runs the strict format/check/test/clippy/build gate.
./packaging/build-linux.sh
mkdir -p "$DIST"

# Windows x64 standalone executable. GNU/MinGW target is intentionally used so
# the cross-linker can run on Debian; Windows-native end-to-end tests remain needed.
need x86_64-w64-mingw32-gcc
rustup target add x86_64-pc-windows-gnu
WINDOWS_BIN="$ROOT/target/x86_64-pc-windows-gnu/release/ghcap.exe"
RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static" \
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    cargo build --release --locked --target x86_64-pc-windows-gnu
[ -s "$WINDOWS_BIN" ] || { echo "ERROR: Windows executable was not created." >&2; exit 1; }
cp -f "$WINDOWS_BIN" "$DIST/ghcap-v$RELEASE_VERSION-windows-x64.exe"

WIN_STAGE="$ROOT/.packaging/windows"
rm -rf "$WIN_STAGE"
mkdir -p "$WIN_STAGE"
cp "$WINDOWS_BIN" "$WIN_STAGE/ghcap.exe"
cp README.md LICENSE DEPENDENCIES.md "$WIN_STAGE/"
cat > "$WIN_STAGE/INSTALL.txt" <<'EOF'
ghcap Windows x64 portable binary
Requires Git for Windows, GitHub CLI (gh), and Gpg4win (gpg) for normal TUI use.
Run ghcap.exe from a terminal. Add the containing folder to PATH if desired.
EOF
(cd "$WIN_STAGE" && zip -q -9 -r "$DIST/ghcap-v$RELEASE_VERSION-windows-x64.zip" .)

# Generate winget manifests only after Windows artifact hashes are known.
GITHUB_REPOSITORY="$GITHUB_REPOSITORY" \
WINGET_PACKAGE_ID="${WINGET_PACKAGE_ID:-ghcap.ghcap}" \
    ./packaging/generate-package-manifests.sh

# Inventory every distributable under dist, including generated package metadata,
# while excluding checksum files themselves to avoid a recursive checksum.
(
    cd "$DIST"
    find . -type f ! -name 'SHA256SUMS' ! -name '*.sha256' -print0 \
      | sort -z | xargs -0 sha256sum > SHA256SUMS
)

# Keep a sidecar checksum for each top-level binary/archive too.
for artifact in "$DIST"/*.exe "$DIST"/*.zip "$DIST"/*.tar.gz "$DIST"/*.deb "$DIST"/*.rpm "$DIST"/*.AppImage "$DIST"/*.pkg.tar.zst; do
    [ -f "$artifact" ] || continue
    (cd "$DIST" && sha256sum "$(basename "$artifact")" > "$(basename "$artifact").sha256")
done

printf '\nAll generated artifacts and package-manager metadata are in %s\n' "$DIST"
cat "$DIST/SHA256SUMS"
