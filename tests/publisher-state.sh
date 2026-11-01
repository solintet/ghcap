#!/usr/bin/env bash
# EN: Simulate GitHub state to prove that the publisher rejects protected tags/releases.
# JA: GitHubの状態を模擬し、公開済みRelease・異なるタグへの変更を拒否することを検査します。
set -euo pipefail
PROJECT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
for cmd in git gh_dummy zip unzip tar jq sha256sum mktemp; do
  [[ "$cmd" == gh_dummy ]] && continue
  command -v "$cmd" >/dev/null 2>&1 || { echo "Missing test tool: $cmd" >&2; exit 127; }
done
BASE=$(mktemp -d "${TMPDIR:-/tmp}/ghcap-release-state.XXXXXXXX")
trap 'rm -rf -- "$BASE"' EXIT
ROOT="$BASE/project"
mkdir -p "$ROOT/packaging" "$ROOT/release-assets" "$BASE/bin" "$BASE/mac" "$BASE/win" "$BASE/linux/ghcap-v0.0.31.0-linux-x86_64/documentation"
cp "$PROJECT/packaging/publish-release.sh" "$PROJECT/packaging/validate-release-assets.sh" "$ROOT/packaging/"
printf 'Test draft notes\n' > "$ROOT/RELEASE_NOTES_v0.0.31.0.md"
printf 'Test preview notes\n' > "$ROOT/packaging/prerelease-notes.md"
(cd "$ROOT" && git init -q -b master && git add packaging RELEASE_NOTES_v0.0.31.0.md && git -c user.name='Test' -c user.email='test@example.invalid' commit -qm 'test fixture')
COMMIT=$(git -C "$ROOT" rev-parse HEAD)
printf '%s\n' "$COMMIT" > "$BASE/mac/SOURCE_COMMIT.txt"
printf '%s\n' "$COMMIT" > "$BASE/win/SOURCE_COMMIT.txt"
printf '%s\n' "$COMMIT" > "$BASE/linux/ghcap-v0.0.31.0-linux-x86_64/documentation/SOURCE_COMMIT.txt"
printf 'test executable\n' > "$BASE/win/ghcap.exe"
DIST="$ROOT/release-assets"
cp "$BASE/win/ghcap.exe" "$DIST/ghcap-v0.0.31.0-windows-x64.exe"
(cd "$BASE/win" && zip -q "$DIST/ghcap-v0.0.31.0-windows-x64.zip" SOURCE_COMMIT.txt ghcap.exe)
for arch in macos-intel macos-apple-silicon; do tar -C "$BASE/mac" -czf "$DIST/ghcap-v0.0.31.0-$arch.tar.gz" SOURCE_COMMIT.txt; done
tar -C "$BASE/linux" -czf "$DIST/ghcap-v0.0.31.0-linux-x86_64.tar.gz" ghcap-v0.0.31.0-linux-x86_64
for name in ghcap_0.0.31.0-1_amd64.deb ghcap-0.0.31.0-1.x86_64.rpm ghcap-0.0.31.0-1-x86_64.pkg.tar.zst ghcap-v0.0.31.0-linux-x86_64.AppImage; do printf 'fixture\n' > "$DIST/$name"; done
for p in "$DIST"/*; do (cd "$DIST" && sha256sum "${p##*/}" > "${p##*/}.sha256"); done
cat > "$BASE/bin/gh" <<'MOCK'
#!/usr/bin/env bash
# EN: Emit responses for read-only API calls and log all writes.
# JA: 読み取りAPIを模擬し、変更APIは記録して検査します。
set -euo pipefail
printf '%s\n' "$*" >> "$MOCK_LOG"
args="$*"
case "$args" in
  *"git/matching-refs/tags/"*)
    printf '[{"ref":"refs/tags/%s","object":{"sha":"%s"}}]\n' "$MOCK_TAG" "$MOCK_SHA"
    ;;
  *"commits/"*)
    if [[ "$args" == *"commits/$MOCK_TAG"* ]]; then
      printf '%s\n' "$MOCK_TAG_COMMIT"
    else
      printf '%s\n' "$MOCK_SHA"
    fi
    ;;
  *"releases?per_page=100"*)
    printf '[[{"tag_name":"%s","draft":false,"prerelease":false,"assets":[]}]]\n' "$MOCK_TAG"
    ;;
  *)
    echo 'Unexpected GitHub mock API request / 不明なAPI操作' >&2
    exit 99
    ;;
esac
MOCK
chmod 755 "$BASE/bin/gh"
export PATH="$BASE/bin:$PATH"
export GITHUB_REPOSITORY=solintet/ghcap GITHUB_SHA="$COMMIT" GITHUB_RUN_ID=123456
export MOCK_SHA="$COMMIT" MOCK_LOG="$BASE/gh-calls.log"
export GHCAP_RELEASE_MODE=stable GITHUB_REF=refs/tags/v0.0.31.0 MOCK_TAG=v0.0.31.0 MOCK_TAG_COMMIT="$COMMIT"
if bash "$ROOT/packaging/publish-release.sh" > "$BASE/stable-output" 2>&1; then
  echo 'FAIL: published Stable Release was modified' >&2; exit 1
fi
if grep -E 'release (create|upload|edit)|--method POST' "$MOCK_LOG" >/dev/null; then
  echo 'FAIL: a forbidden GitHub write occurred' >&2; exit 1
fi
printf 'PASS / 成功: published Stable release protected\n'
rm -f "$DIST/ghcap-v0.0.31.0-source.tar.gz" "$DIST/ghcap-v0.0.31.0-source.tar.gz.sha256" "$DIST/SHA256SUMS"
: > "$MOCK_LOG"
export GHCAP_RELEASE_MODE=preview GITHUB_REF=refs/heads/master MOCK_TAG="v0.0.31.0-pre.${COMMIT:0:12}" MOCK_TAG_COMMIT=2222222222222222222222222222222222222222
if bash "$ROOT/packaging/publish-release.sh" > "$BASE/preview-output" 2>&1; then
  echo 'FAIL: incompatible existing tag was accepted' >&2; exit 1
fi
if grep -E 'release (create|upload|edit)|--method POST' "$MOCK_LOG" >/dev/null; then
  echo 'FAIL: a forbidden GitHub write occurred' >&2; exit 1
fi
printf 'PASS / 成功: mismatched Git tag protected\n'
echo 'Publisher state tests passed / Release状態テスト2件成功'
