#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

RELEASE_VERSION="0.0.31.0"
CARGO_VERSION="0.31.0"
DIST="$ROOT/dist"
ARCH_RAW="$(dpkg --print-architecture 2>/dev/null || uname -m)"
case "$ARCH_RAW" in
    amd64|x86_64) ARCH="x86_64"; RPM_ARCH="x86_64"; DEB_ARCH="amd64"; ARCH_PKG="x86_64" ;;
    arm64|aarch64) ARCH="aarch64"; RPM_ARCH="aarch64"; DEB_ARCH="arm64"; ARCH_PKG="aarch64" ;;
    armhf|armv7l) ARCH="armv7"; RPM_ARCH="armv7hl"; DEB_ARCH="armhf"; ARCH_PKG="armv7h" ;;
    *) echo "ERROR: Unsupported host architecture: $ARCH_RAW" >&2; exit 1 ;;
esac

need() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "ERROR: Required command '$1' is missing." >&2
        echo "Install the Debian package that provides it, then rerun this script." >&2
        exit 1
    }
}

for command in cargo rustc tar sha256sum dpkg-buildpackage dpkg-deb rpmbuild zstd curl; do
    need "$command"
done
cargo clippy --version >/dev/null 2>&1 || {
    echo "ERROR: Clippy is missing. Install with: rustup component add clippy" >&2
    exit 1
}

# EN: Never download or execute an unverified external packager during local builds.
# JA: ローカルビルド時に未検証の外部パッケージツールを取得・実行しません。
APPIMAGE_TOOL="${APPIMAGETOOL:-}"
if [ -z "$APPIMAGE_TOOL" ]; then
    APPIMAGE_TOOL=$(command -v appimagetool 2>/dev/null || true)
fi
[ -n "$APPIMAGE_TOOL" ] && [ -x "$APPIMAGE_TOOL" ] || {
    echo "ERROR: Set APPIMAGETOOL to an approved local appimagetool." >&2; exit 1;
}
if [ "$ARCH" = x86_64 ]; then
    EXPECTED_APPIMAGE_HASH="${APPIMAGETOOL_SHA256:-ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0}"
else
    EXPECTED_APPIMAGE_HASH="${APPIMAGETOOL_SHA256:-}"
fi
[ -n "$EXPECTED_APPIMAGE_HASH" ] || { echo "ERROR: Set APPIMAGETOOL_SHA256 for this architecture." >&2; exit 1; }
printf '%s  %s\n' "$EXPECTED_APPIMAGE_HASH" "$APPIMAGE_TOOL" | sha256sum -c --status || {
    echo 'ERROR: AppImage tool hash mismatch / 外部ツールのチェックサム不一致' >&2; exit 1;
}

# Build Arch package on Debian using an Arch Linux container. Docker or Podman is required.
CONTAINER_ENGINE=""
if command -v podman >/dev/null 2>&1; then
    CONTAINER_ENGINE="podman"
elif command -v docker >/dev/null 2>&1; then
    CONTAINER_ENGINE="docker"
else
    echo "ERROR: Arch package creation requires Podman or Docker to run makepkg in Arch Linux." >&2
    echo "Install one on Debian (e.g. podman or docker.io), ensure it can run containers, then retry." >&2
    exit 1
fi

if [ ! -f Cargo.lock ]; then
    echo "Cargo.lock is missing; generating it (network access may be required)..."
    cargo generate-lockfile
fi
grep -q "^version = \"$CARGO_VERSION\"$" Cargo.toml || {
    echo "ERROR: Cargo.toml version must be $CARGO_VERSION." >&2; exit 1;
}
grep -q "^ghcap ($RELEASE_VERSION-1)" debian/changelog || {
    echo "ERROR: debian/changelog must start with ghcap ($RELEASE_VERSION-1)." >&2; exit 1;
}

mkdir -p "$DIST"
# EN: Never delete or overwrite a previous distribution artifact automatically.
# JA: 既存の配布ファイルを自動削除・上書きしません。
for candidate in "ghcap-$RELEASE_VERSION-linux-$ARCH.tar.gz" "ghcap-$RELEASE_VERSION-linux-$ARCH.AppImage" "ghcap-$RELEASE_VERSION-1.$RPM_ARCH.rpm" "ghcap-$RELEASE_VERSION-1-$ARCH_PKG.pkg.tar.zst" "ghcap_$RELEASE_VERSION-1_$DEB_ARCH.deb"; do
    [ ! -e "$DIST/$candidate" ] || { echo "ERROR: Refusing to overwrite $candidate" >&2; exit 1; }
done

# Shared strict release gate. Formatting is applied first because prior source archives
# may contain rustfmt-only differences; the subsequent check ensures the tree is clean.
cargo fmt --all -- --check
cargo check --locked --all-targets --all-features
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --release --locked

BIN="$ROOT/target/release/ghcap"
[ -x "$BIN" ] || { echo "ERROR: Missing release binary: $BIN" >&2; exit 1; }

# 1) Generic Linux tar.gz.
STAGE="$DIST/ghcap-$RELEASE_VERSION-linux-$ARCH"
rm -rf "$STAGE"
mkdir -p "$STAGE"
install -m 0755 "$BIN" "$STAGE/ghcap"
install -m 0644 README.md LICENSE DEPENDENCIES.md "$STAGE/"
cat > "$STAGE/INSTALL.txt" <<'EOF'
ghcap Linux binary archive
Requires git, GitHub CLI (gh), and GnuPG (gpg).
Run ./ghcap, or install it with:
  mkdir -p "$HOME/.local/bin"
  install -m 0755 ./ghcap "$HOME/.local/bin/ghcap"
This binary is built for the architecture in the archive filename.
EOF
ARCHIVE="$DIST/ghcap-$RELEASE_VERSION-linux-$ARCH.tar.gz"
tar -C "$DIST" -czf "$ARCHIVE" "$(basename "$STAGE")"
(cd "$DIST" && sha256sum "$(basename "$ARCHIVE")" > "$(basename "$ARCHIVE").sha256")
tar -tzf "$ARCHIVE" | grep -q "^ghcap-$RELEASE_VERSION-linux-$ARCH/ghcap$"

# 2) Debian package.
dpkg-buildpackage -us -uc -b
DEB_SOURCE="$(dirname "$ROOT")/ghcap_${RELEASE_VERSION}-1_${DEB_ARCH}.deb"
[ -f "$DEB_SOURCE" ] || { echo "ERROR: Debian package not found: $DEB_SOURCE" >&2; exit 1; }
cp -f "$DEB_SOURCE" "$DIST/"
dpkg-deb --info "$DIST/$(basename "$DEB_SOURCE")" >/dev/null

# 3) RPM package via rpmbuild on Debian.
RPMROOT="$ROOT/.packaging/rpmbuild"
rm -rf "$RPMROOT"
mkdir -p "$RPMROOT/BUILD" "$RPMROOT/BUILDROOT" "$RPMROOT/RPMS" \
         "$RPMROOT/SOURCES" "$RPMROOT/SPECS" "$RPMROOT/SRPMS"
cp "$BIN" "$RPMROOT/SOURCES/ghcap"
cat > "$RPMROOT/SPECS/ghcap.spec" <<EOF
Name:           ghcap
Version:        $RELEASE_VERSION
Release:        1
Summary:        GitHub workflow terminal UI
License:        BSD-2-Clause
BuildArch:      $RPM_ARCH
Requires:       git
Requires:       gh
Requires:       gnupg2
%description
Cross-platform terminal UI for GitHub account, repository, commit, push,
GPG presets, and repository-specific pre-push workflows.
%install
mkdir -p %{buildroot}%{_bindir}
install -m 0755 %{_sourcedir}/ghcap %{buildroot}%{_bindir}/ghcap
%files
%{_bindir}/ghcap
EOF
rpmbuild --define "_topdir $RPMROOT" -bb "$RPMROOT/SPECS/ghcap.spec"
RPM_SOURCE="$RPMROOT/RPMS/$RPM_ARCH/ghcap-$RELEASE_VERSION-1.$RPM_ARCH.rpm"
[ -f "$RPM_SOURCE" ] || { echo "ERROR: RPM package not found: $RPM_SOURCE" >&2; exit 1; }
cp "$RPM_SOURCE" "$DIST/"
rpm -qpi "$DIST/$(basename "$RPM_SOURCE")" >/dev/null

# 4) AppImage. It is a CLI AppImage; system dependencies git/gh/gpg remain external.
APPDIR="$ROOT/.packaging/AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps"
install -m 0755 "$BIN" "$APPDIR/usr/bin/ghcap"
cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/ghcap" "$@"
EOF
chmod 0755 "$APPDIR/AppRun"
cat > "$APPDIR/ghcap.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=ghcap
Comment=GitHub workflow terminal UI
Icon=ghcap
Exec=ghcap
Terminal=true
Categories=Development;Utility;
EOF
cp "$APPDIR/ghcap.desktop" "$APPDIR/usr/share/applications/ghcap.desktop"
cat > "$APPDIR/ghcap.svg" <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
<rect x="12" y="12" width="232" height="232" rx="24" fill="#222222"/>
<path d="M56 68h144v22H56zm0 49h144v22H56zm0 49h96v22H56z" fill="#ffffff"/>
</svg>
EOF
cp "$APPDIR/ghcap.svg" "$APPDIR/usr/share/icons/hicolor/256x256/apps/ghcap.svg"
APPIMAGE="$DIST/ghcap-$RELEASE_VERSION-linux-$ARCH.AppImage"
ARCH="$(uname -m)"
APPIMAGE_ARCH="$ARCH"
[ "$APPIMAGE_ARCH" = "x86_64" ] && APPIMAGE_ARCH="x86_64"
[ "$APPIMAGE_ARCH" = "aarch64" ] && APPIMAGE_ARCH="aarch64"
APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGE_TOOL" --no-appstream "$APPDIR" "$APPIMAGE"
[ -s "$APPIMAGE" ] || { echo "ERROR: AppImage was not created." >&2; exit 1; }
chmod 0755 "$APPIMAGE"

# 5) Arch package. Build in an Arch container to use the official makepkg toolchain.
ARCH_STAGE="$ROOT/.packaging/arch"
rm -rf "$ARCH_STAGE"
mkdir -p "$ARCH_STAGE"
cp "$BIN" "$ARCH_STAGE/ghcap"
cp README.md LICENSE DEPENDENCIES.md "$ARCH_STAGE/"
cat > "$ARCH_STAGE/PKGBUILD" <<EOF
pkgname=ghcap
pkgver=$RELEASE_VERSION
pkgrel=1
pkgdesc='GitHub workflow terminal UI'
arch=('$ARCH_PKG')
url='https://github.com/'
license=('BSD-2-Clause')
depends=('git' 'github-cli' 'gnupg')
source=('ghcap' 'README.md' 'LICENSE' 'DEPENDENCIES.md')
sha256sums=('SKIP' 'SKIP' 'SKIP' 'SKIP')
package() {
  install -Dm755 "\$srcdir/ghcap" "\$pkgdir/usr/bin/ghcap"
  install -Dm644 "\$srcdir/README.md" "\$pkgdir/usr/share/doc/ghcap/README.md"
  install -Dm644 "\$srcdir/LICENSE" "\$pkgdir/usr/share/licenses/ghcap/LICENSE"
  install -Dm644 "\$srcdir/DEPENDENCIES.md" "\$pkgdir/usr/share/doc/ghcap/DEPENDENCIES.md"
}
EOF
# makepkg refuses to run as root; use a disposable Arch container and a non-root build user.
# Mount only the staging directory. Output is copied back to dist afterwards.
if [ "$CONTAINER_ENGINE" = "podman" ]; then
    "$CONTAINER_ENGINE" run --rm -v "$ARCH_STAGE:/build:Z" -w /build archlinux:latest \
      bash -lc 'pacman -Sy --noconfirm --needed base-devel git github-cli gnupg; useradd -m builder || true; chown -R builder:builder /build; su builder -c "cd /build && makepkg --noconfirm --cleanbuild"'
else
    "$CONTAINER_ENGINE" run --rm -v "$ARCH_STAGE:/build" -w /build archlinux:latest \
      bash -lc 'pacman -Sy --noconfirm --needed base-devel git github-cli gnupg; useradd -m builder || true; chown -R builder:builder /build; su builder -c "cd /build && makepkg --noconfirm --cleanbuild"'
fi
ARCH_PACKAGE="$(find "$ARCH_STAGE" -maxdepth 1 -type f -name 'ghcap-*.pkg.tar.zst' -print -quit)"
[ -n "$ARCH_PACKAGE" ] && [ -f "$ARCH_PACKAGE" ] || { echo "ERROR: Arch package was not created." >&2; exit 1; }
cp "$ARCH_PACKAGE" "$DIST/"
zstd -t "$DIST/$(basename "$ARCH_PACKAGE")"

# Checksums for all final deliverables.
(cd "$DIST" && sha256sum ghcap-"$RELEASE_VERSION"-linux-"$ARCH".AppImage ghcap-"$RELEASE_VERSION"-1."$RPM_ARCH".rpm ghcap_"$RELEASE_VERSION"-1_"$DEB_ARCH".deb "$(basename "$ARCH_PACKAGE")" > SHA256SUMS)

printf '\nAll Linux package outputs are in %s:\n' "$DIST"
find "$DIST" -maxdepth 1 -type f \( -name "*.tar.gz" -o -name "*.AppImage" -o -name "*.rpm" -o -name "*.deb" -o -name "*.pkg.tar.zst" -o -name "SHA256SUMS" -o -name "*.sha256" \) -printf '  %f\n' | sort
