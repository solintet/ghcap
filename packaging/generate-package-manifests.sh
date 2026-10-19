#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
RELEASE_VERSION="0.0.29.0"
DIST="$ROOT/dist"
REPO=${GITHUB_REPOSITORY:-}
WINGET_ID=${WINGET_PACKAGE_ID:-ghcap.ghcap}
[ -n "$REPO" ] || { echo "ERROR: Set GITHUB_REPOSITORY=OWNER/REPO." >&2; exit 1; }
case "$REPO" in */*) ;; *) echo "ERROR: GITHUB_REPOSITORY must be OWNER/REPO." >&2; exit 1 ;; esac
WIN_ZIP="ghcap-v$RELEASE_VERSION-windows-x64.zip"
[ -s "$DIST/$WIN_ZIP" ] || { echo "ERROR: Required release artifact missing: $DIST/$WIN_ZIP" >&2; exit 1; }
WIN_SHA=$(sha256sum "$DIST/$WIN_ZIP" | awk '{print toupper($1)}')
BASE_URL="https://github.com/$REPO/releases/download/v$RELEASE_VERSION"
WINGET_DIR="$ROOT/packaging/winget"
META_DIR="$DIST/manifests"
mkdir -p "$WINGET_DIR" "$META_DIR"
cat > "$WINGET_DIR/$WINGET_ID.yaml" <<EOF
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.version.1.10.0.schema.json
PackageIdentifier: $WINGET_ID
PackageVersion: $RELEASE_VERSION
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.10.0
EOF
cat > "$WINGET_DIR/$WINGET_ID.locale.en-US.yaml" <<EOF
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.defaultLocale.1.10.0.schema.json
PackageIdentifier: $WINGET_ID
PackageVersion: $RELEASE_VERSION
PackageLocale: en-US
Publisher: ghcap contributors
PackageName: ghcap
License: BSD-2-Clause
ShortDescription: Cross-platform GitHub workflow terminal UI
Description: >-
  A terminal UI for GitHub account and repository workflows, commit presets,
  push operations, and GPG integration. Git, GitHub CLI, and GnuPG are separate
  runtime dependencies.
ManifestType: defaultLocale
ManifestVersion: 1.10.0
EOF
cat > "$WINGET_DIR/$WINGET_ID.installer.yaml" <<EOF
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.installer.1.10.0.schema.json
PackageIdentifier: $WINGET_ID
PackageVersion: $RELEASE_VERSION
InstallerType: zip
NestedInstallerType: portable
Commands:
  - ghcap
Installers:
  - Architecture: x64
    NestedInstallerFiles:
      - RelativeFilePath: ghcap.exe
        PortableCommandAlias: ghcap
    InstallerUrl: $BASE_URL/$WIN_ZIP
    InstallerSha256: $WIN_SHA
ManifestType: installer
ManifestVersion: 1.10.0
EOF
cp "$WINGET_DIR/$WINGET_ID.yaml" "$META_DIR/"
cp "$WINGET_DIR/$WINGET_ID.locale.en-US.yaml" "$META_DIR/"
cp "$WINGET_DIR/$WINGET_ID.installer.yaml" "$META_DIR/"
printf 'Generated winget manifests: %s\n' "$WINGET_DIR"
