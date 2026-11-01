#!/usr/bin/env bash
# EN: Package one already-tested x86_64 binary. No source rebuild or system modification.
# JA: 検証済みのx86_64バイナリから配布物を生成します。再ビルドやシステム変更は行いません。
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT"
VERSION=0.0.31.0
BIN="$ROOT/target/release/ghcap"
DIST="$ROOT/dist"

fail() { printf 'ERROR / エラー: %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || fail "Missing command / コマンド不足: $1"; }
for tool in git tar sha256sum dpkg-deb rpmbuild rpm bsdtar zstd install mktemp stat; do need "$tool"; done
[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || fail 'Linux x86_64 only / Linux x86_64専用'
[[ -x "$BIN" ]] || fail "Missing release binary / 実行ファイルがありません: $BIN"
[[ $("$BIN" --version) == "ghcap v$VERSION" ]] || fail 'Binary version mismatch / バージョン不一致'
COMMIT=$(git rev-parse --verify HEAD)
EPOCH=$(git show -s --format=%ct HEAD)
[[ $COMMIT =~ ^[a-f0-9]{40}$ ]] || fail 'Invalid Git SHA / Git SHAが不正'
export SOURCE_DATE_EPOCH="$EPOCH"
mkdir -p "$DIST"
EXPECTED=("ghcap-v${VERSION}-linux-x86_64.tar.gz" "ghcap_${VERSION}-1_amd64.deb" "ghcap-${VERSION}-1.x86_64.rpm" "ghcap-${VERSION}-1-x86_64.pkg.tar.zst" "ghcap-v${VERSION}-linux-x86_64.AppImage")
for filename in "${EXPECTED[@]}"; do [[ ! -e "$DIST/$filename" ]] || fail "Refusing overwrite / 上書き禁止: $filename"; done
WORK=$(mktemp -d "${TMPDIR:-/tmp}/ghcap-pkg.XXXXXXXX")
trap 'rm -rf -- "$WORK"' EXIT
mkdir -p "$WORK/payload/usr/bin" "$WORK/payload/usr/share/doc/ghcap"
install -m 755 "$BIN" "$WORK/payload/usr/bin/ghcap"
DOCS="$WORK/payload/usr/share/doc/ghcap"
for filename in README.md LICENSE DEPENDENCIES.md THIRD_PARTY_NOTICES.md; do cp -- "$ROOT/$filename" "$DOCS/$filename"; done
cp -a -- THIRD_PARTY_LICENSES "$DOCS/THIRD_PARTY_LICENSES"
printf '%s\n' "$COMMIT" > "$DOCS/SOURCE_COMMIT.txt"
{ rustc -Vv; printf '\nCargo.lock SHA-256: '; sha256sum Cargo.lock | cut -d ' ' -f 1; printf 'Linux x86_64, Ubuntu 22.04 build; glibc >= 2.35.\n'; } > "$DOCS/BUILD_INFO.txt"

# EN: Portable tarball contains its own provenance and the upstream notices.
# JA: tar.gzにもソース履歴と第三者ライセンスを収録します。
STAGE="$WORK/ghcap-v${VERSION}-linux-x86_64"
mkdir -p "$STAGE/documentation"
cp "$BIN" "$STAGE/ghcap"
cp -a "$DOCS/." "$STAGE/documentation/"
printf 'Run ./ghcap from a terminal. Git and compatible GitHub CLI are external dependencies.\nターミナルから ./ghcap を起動します。Git・対応版ghは別途必要です。\n' > "$STAGE/INSTALL.txt"
tar -C "$WORK" -czf "$DIST/${EXPECTED[0]}" "$(basename "$STAGE")"
tar -tzf "$DIST/${EXPECTED[0]}" > "$WORK/tar-contents.txt"
grep -Fxq "$(basename "$STAGE")/documentation/SOURCE_COMMIT.txt" "$WORK/tar-contents.txt" || fail 'Invalid tarball / tar.gzの内容が不正'

# EN: Debian uses explicit binary payload and does not silently install system packages.
# JA: Debian版は既存の実行ファイルを梱包し、生成時に依存ソフトを導入しません。
cp -a "$WORK/payload" "$WORK/deb"
mkdir -p "$WORK/deb/DEBIAN"
cat > "$WORK/deb/DEBIAN/control" <<CONTROL
Package: ghcap
Version: ${VERSION}-1
Architecture: amd64
Maintainer: solintet
Section: utils
Priority: optional
Depends: libc6 (>= 2.35), libgcc-s1, git
Description: Interactive Git and GitHub terminal interface
 ghcap provides terminal-based repository and GitHub workflows.
 GitHub CLI is required for GitHub features; GnuPG is required only for GPG signing.
CONTROL
dpkg-deb --root-owner-group --build "$WORK/deb" "$DIST/${EXPECTED[1]}"
dpkg-deb --info "$DIST/${EXPECTED[1]}" >/dev/null
mkdir -p "$WORK/deb-check"
dpkg-deb --extract "$DIST/${EXPECTED[1]}" "$WORK/deb-check"
[[ $("$WORK/deb-check/usr/bin/ghcap" --version) == "ghcap v$VERSION" ]] || fail 'deb payload mismatch / debに不整合があります'

# EN: Build an RPM from the exact same binary payload, retaining RPM dependency scanning.
# JA: 同一バイナリからRPMを生成し、RPM標準の依存関係検査を利用します。
RPMROOT="$WORK/rpm"
mkdir -p "$RPMROOT"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
cat > "$RPMROOT/SPECS/ghcap.spec" <<SPEC
Name: ghcap
Version: ${VERSION}
Release: 1
Summary: Interactive Git and GitHub terminal interface
License: BSD-2-Clause
URL: https://github.com/solintet/ghcap
BuildArch: x86_64
Requires: git
%description
Terminal-based Git and GitHub workflows published by solintet.
GitHub CLI is needed for GitHub features.
%install
mkdir -p %{buildroot}/usr
cp -a ${WORK}/payload/usr/. %{buildroot}/usr/
%files
/usr/bin/ghcap
/usr/share/doc/ghcap
SPEC
rpmbuild --define "_topdir $RPMROOT" -bb "$RPMROOT/SPECS/ghcap.spec"
cp -- "$RPMROOT/RPMS/x86_64/ghcap-${VERSION}-1.x86_64.rpm" "$DIST/${EXPECTED[2]}"
rpm -qp --info "$DIST/${EXPECTED[2]}" >/dev/null

# EN: A Pacman package is made from a documented .PKGINFO and one source payload.
# JA: Pacmanパッケージは検証可能な.PKGINFOと同一ペイロードから生成します。
cp -a "$WORK/payload" "$WORK/pacman"
SIZE=$(find "$WORK/pacman" -type f -printf '%s\n' | awk '{s+=$1}END{printf "%.0f",s}')
cat > "$WORK/pacman/.PKGINFO" <<PKG
pkgname = ghcap
pkgbase = ghcap
pkgver = ${VERSION}-1
pkgdesc = Interactive Git and GitHub terminal interface
url = https://github.com/solintet/ghcap
builddate = ${EPOCH}
packager = solintet
size = ${SIZE}
arch = x86_64
license = BSD-2-Clause
depend = glibc>=2.35
depend = gcc-libs
depend = git
optdepend = github-cli: GitHub integration
optdepend = gnupg: GPG signing
PKG
bsdtar --uid 0 --gid 0 -cf "$WORK/pacman.tar" -C "$WORK/pacman" .PKGINFO usr
zstd -q -19 "$WORK/pacman.tar" -o "$DIST/${EXPECTED[3]}"
bsdtar -tf "$DIST/${EXPECTED[3]}" > "$WORK/pacman-contents.txt"
grep -Eq '^(\./)?usr/bin/ghcap$' "$WORK/pacman-contents.txt" || fail 'Invalid Pacman payload / Pacmanに不整合があります'

# EN: Refuse unverified AppImage tooling; pin official tool and runtime digests.
# JA: 署名未確認のツールは使用せず、公式配布物のSHA-256を固定します。
TOOL_SHA=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
RUNTIME_SHA=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
APPIMAGETOOL=${APPIMAGETOOL:-"$WORK/appimagetool-x86_64.AppImage"}
APPIMAGE_RUNTIME=${APPIMAGE_RUNTIME:-"$WORK/runtime-x86_64"}
if [[ ! -f "$APPIMAGETOOL" || ! -f "$APPIMAGE_RUNTIME" ]]; then
  need aria2c
  aria2c -x16 -s16 -k1M "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage" --dir="$WORK" --out=appimagetool-x86_64.AppImage --allow-overwrite=false
  aria2c -x16 -s16 -k1M "https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64" --dir="$WORK" --out=runtime-x86_64 --allow-overwrite=false
fi
printf '%s  %s\n' "$TOOL_SHA" "$APPIMAGETOOL" | sha256sum --check --status || fail 'AppImage tool digest mismatch / ツールの検証に失敗'
printf '%s  %s\n' "$RUNTIME_SHA" "$APPIMAGE_RUNTIME" | sha256sum --check --status || fail 'AppImage runtime digest mismatch / ランタイムの検証に失敗'
chmod 755 "$APPIMAGETOOL"
cp -a "$WORK/payload" "$WORK/AppDir"
cat > "$WORK/AppDir/AppRun" <<'APPRUN'
#!/bin/sh
HERE=$(dirname "$(readlink -f "$0")")
exec "$HERE/usr/bin/ghcap" "$@"
APPRUN
chmod 755 "$WORK/AppDir/AppRun"
cat > "$WORK/AppDir/ghcap.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=ghcap
Comment=Git and GitHub terminal interface
Exec=ghcap
Icon=ghcap
Terminal=true
Categories=Development;Utility;
DESKTOP
cat > "$WORK/AppDir/ghcap.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#222"/><path d="M12 18h40M12 32h40M12 46h28" stroke="white" stroke-width="5"/></svg>
SVG
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL" --runtime-file "$APPIMAGE_RUNTIME" --no-appstream "$WORK/AppDir" "$DIST/${EXPECTED[4]}"
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$DIST/${EXPECTED[4]}" --version | grep -Fx "ghcap v$VERSION"

for filename in "${EXPECTED[@]}"; do
  [[ -s "$DIST/$filename" ]] || fail "Empty artifact / 成果物が空: $filename"
  ( cd "$DIST" && sha256sum "$filename" > "$filename.sha256" )
done
printf 'Packages generated / 配布物生成完了: %s\n' "${EXPECTED[*]}"
