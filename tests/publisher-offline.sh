#!/usr/bin/env bash
# EN: Exercise the independent publisher artifact gate without network or credentials.
# JA: 認証・ネットワークを使わず、公開前の成果物検査の成功・失敗経路を確認します。
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
for tool in zip unzip tar sha256sum mktemp; do command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 127; }; done
BASE=$(mktemp -d "${TMPDIR:-/tmp}/ghcap-test.XXXXXXXX")
trap 'rm -rf -- "$BASE"' EXIT
DIST="$BASE/dist"
mkdir -p "$DIST" "$BASE/mac" "$BASE/linux/ghcap-v0.0.31.0-linux-x86_64/documentation" "$BASE/windows"
COMMIT=1111111111111111111111111111111111111111
printf '%s\n' "$COMMIT" > "$BASE/mac/SOURCE_COMMIT.txt"
printf '%s\n' "$COMMIT" > "$BASE/linux/ghcap-v0.0.31.0-linux-x86_64/documentation/SOURCE_COMMIT.txt"
printf '%s\n' "$COMMIT" > "$BASE/windows/SOURCE_COMMIT.txt"
printf 'test native executable\n' > "$BASE/windows/ghcap.exe"
cp "$BASE/windows/ghcap.exe" "$DIST/ghcap-v0.0.31.0-windows-x64.exe"
tar -C "$BASE/mac" -czf "$DIST/ghcap-v0.0.31.0-macos-intel.tar.gz" SOURCE_COMMIT.txt
tar -C "$BASE/mac" -czf "$DIST/ghcap-v0.0.31.0-macos-apple-silicon.tar.gz" SOURCE_COMMIT.txt
tar -C "$BASE/linux" -czf "$DIST/ghcap-v0.0.31.0-linux-x86_64.tar.gz" ghcap-v0.0.31.0-linux-x86_64
(cd "$BASE/windows" && zip -q "$DIST/ghcap-v0.0.31.0-windows-x64.zip" SOURCE_COMMIT.txt ghcap.exe)
for name in ghcap_0.0.31.0-1_amd64.deb ghcap-0.0.31.0-1.x86_64.rpm ghcap-0.0.31.0-1-x86_64.pkg.tar.zst ghcap-v0.0.31.0-linux-x86_64.AppImage; do
  printf 'test fixture\n' > "$DIST/$name"
done
for path in "$DIST"/*; do (cd "$DIST" && sha256sum "${path##*/}" > "${path##*/}.sha256"); done
"$ROOT/packaging/validate-release-assets.sh" "$DIST" "$COMMIT" >/dev/null
printf 'PASS / 成功: valid artifacts\n'
# EN: Mutating one byte after checksum creation must be rejected.
# JA: チェックサム作成後の内容変更は必ず拒否されます。
printf 'tampered' >> "$DIST/ghcap-v0.0.31.0-windows-x64.exe"
if "$ROOT/packaging/validate-release-assets.sh" "$DIST" "$COMMIT" >/dev/null 2>&1; then
  echo 'FAIL: modified executable was accepted' >&2; exit 1
fi
printf 'PASS / 成功: checksum mismatch rejected\n'
(cd "$DIST" && sha256sum ghcap-v0.0.31.0-windows-x64.exe > ghcap-v0.0.31.0-windows-x64.exe.sha256)
if "$ROOT/packaging/validate-release-assets.sh" "$DIST" "$COMMIT" >/dev/null 2>&1; then
  echo 'FAIL: ZIP/EXE mismatch was accepted' >&2; exit 1
fi
printf 'PASS / 成功: archive-to-binary mismatch rejected\n'
# EN: Any additional file is rejected, including stale artifacts from another run.
# JA: 以前の実行から残った余計な配布ファイルも拒否します。
rm -f "$DIST/ghcap-v0.0.31.0-windows-x64.exe"
cp "$BASE/windows/ghcap.exe" "$DIST/ghcap-v0.0.31.0-windows-x64.exe"
(cd "$DIST" && sha256sum ghcap-v0.0.31.0-windows-x64.exe > ghcap-v0.0.31.0-windows-x64.exe.sha256)
printf old > "$DIST/old-artifact.zip"
if "$ROOT/packaging/validate-release-assets.sh" "$DIST" "$COMMIT" >/dev/null 2>&1; then
  echo 'FAIL: unexpected output accepted' >&2; exit 1
fi
printf 'PASS / 成功: stale artifact rejected\n'
rm -f "$DIST/old-artifact.zip"
if "$ROOT/packaging/validate-release-assets.sh" "$DIST" 2222222222222222222222222222222222222222 >/dev/null 2>&1; then
  echo 'FAIL: wrong source commit accepted' >&2; exit 1
fi
printf 'PASS / 成功: provenance mismatch rejected\n'
echo 'All offline artifact tests passed / オフライン検査4件成功'
