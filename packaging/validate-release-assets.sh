#!/usr/bin/env bash
# EN: Offline, side-effect-free release checks; no GitHub API or authentication is needed.
# JA: GitHub認証を使わず、配布ファイルの個数・出所・整合性を検証します。
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
DIST=${1:-"$ROOT/release-assets"}
COMMIT=${2:-${GITHUB_SHA:-}}
VERSION=0.0.31.0
fail() { printf 'Asset verification failed / 配布物の検証に失敗: %s\n' "$*" >&2; exit 1; }
[[ "$COMMIT" =~ ^[a-fA-F0-9]{40}$ ]] || fail 'Invalid source commit / コミットが不正'
[[ -d "$DIST" ]] || fail 'Asset directory does not exist / 配布ディレクトリがありません'
for tool in sha256sum tar unzip sort cmp; do command -v "$tool" >/dev/null || fail "Missing tool / ツール不足: $tool"; done
expected=(
  "ghcap-v${VERSION}-macos-intel.tar.gz"
  "ghcap-v${VERSION}-macos-apple-silicon.tar.gz"
  "ghcap-v${VERSION}-windows-x64.exe"
  "ghcap-v${VERSION}-windows-x64.zip"
  "ghcap-v${VERSION}-linux-x86_64.tar.gz"
  "ghcap_${VERSION}-1_amd64.deb"
  "ghcap-${VERSION}-1.x86_64.rpm"
  "ghcap-${VERSION}-1-x86_64.pkg.tar.zst"
  "ghcap-v${VERSION}-linux-x86_64.AppImage"
)
# EN: Only the nine expected artifacts and their checksums may exist before publication.
# JA: 公開前は9成果物と対応するチェックサム以外を受け付けません。
shopt -s nullglob
files=("$DIST"/*)
[[ ${#files[@]} -eq 18 ]] || fail "Expected 18 files, found ${#files[@]} / 18ファイル必要"
for name in "${expected[@]}"; do
  [[ -f "$DIST/$name" && ! -L "$DIST/$name" && -s "$DIST/$name" ]] || fail "Artifact missing/invalid / 成果物が不足: $name"
  [[ -f "$DIST/$name.sha256" && ! -L "$DIST/$name.sha256" ]] || fail "Checksum missing / チェックサム不足: $name"
  (cd "$DIST" && sha256sum --check --status -- "$name.sha256") || fail "Checksum differs / ハッシュ不一致: $name"
done
for path in "${files[@]}"; do
  name=${path##*/}
  found=0
  for expected_name in "${expected[@]}"; do
     if [[ "$name" == "$expected_name" || "$name" == "$expected_name.sha256" ]]; then found=1; break; fi
  done
  [[ "$found" == 1 ]] || fail "Unexpected file / 不明なファイル: $name"
done
# EN: Archive provenance must match the GitHub build commit (not just the filename).
# JA: アーカイブに収録されたビルド元コミットがActionsのコミットと一致する必要があります。
for platform in macos-intel macos-apple-silicon; do
  note=$(tar -xOzf "$DIST/ghcap-v${VERSION}-${platform}.tar.gz" SOURCE_COMMIT.txt) || fail "Invalid macOS provenance / macOS出所が不正: $platform"
  [[ "$note" == "$COMMIT" ]] || fail "macOS source commit mismatch: $platform"
done
note=$(tar -xOzf "$DIST/ghcap-v${VERSION}-linux-x86_64.tar.gz" "ghcap-v${VERSION}-linux-x86_64/documentation/SOURCE_COMMIT.txt") || fail 'Invalid Linux provenance'
[[ "$note" == "$COMMIT" ]] || fail 'Linux source commit mismatch'
note=$(unzip -p "$DIST/ghcap-v${VERSION}-windows-x64.zip" SOURCE_COMMIT.txt) || fail 'Invalid Windows provenance'
[[ "$note" == "$COMMIT" ]] || fail 'Windows source commit mismatch'
inside=$(unzip -p "$DIST/ghcap-v${VERSION}-windows-x64.zip" ghcap.exe | sha256sum | cut -d ' ' -f1) || fail 'Invalid ZIP executable'
outside=$(sha256sum "$DIST/ghcap-v${VERSION}-windows-x64.exe" | cut -d ' ' -f1)
[[ "$inside" == "$outside" ]] || fail 'ZIP executable differs from standalone exe / ZIP実行ファイルが異なります'
printf 'Validated 9 artifacts / 9成果物の検証成功: %s\n' "$COMMIT"
