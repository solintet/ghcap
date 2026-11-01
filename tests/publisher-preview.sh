#!/usr/bin/env bash
# EN: Exercise a first-time preview publication with a missing tag and a newly created draft.
# JA: 未作成タグ・新規Draft・21成果物の公開をGitHubに接続せず検証します。
set -euo pipefail
PROJECT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
for cmd in git jq zip unzip tar sha256sum mktemp; do command -v "$cmd" >/dev/null 2>&1 || { echo "Missing test tool: $cmd" >&2; exit 127; }; done
BASE=$(mktemp -d "${TMPDIR:-/tmp}/ghcap-preview-test.XXXXXXXX")
trap 'rm -rf -- "$BASE"' EXIT
ROOT="$BASE/project"
mkdir -p "$ROOT/packaging" "$ROOT/release-assets" "$BASE/bin" "$BASE/mac" "$BASE/win" "$BASE/linux/ghcap-v0.0.31.0-linux-x86_64/documentation" "$BASE/remote-assets"
cp "$PROJECT/packaging/publish-release.sh" "$PROJECT/packaging/validate-release-assets.sh" "$ROOT/packaging/"
printf 'Test release notes\n' > "$ROOT/packaging/prerelease-notes.md"
(cd "$ROOT" && git init -q -b master && git add packaging && git -c user.name='Test' -c user.email='test@example.invalid' commit -qm 'fixture')
COMMIT=$(git -C "$ROOT" rev-parse HEAD)
export GITHUB_REPOSITORY=solintet/ghcap GITHUB_SHA="$COMMIT" GITHUB_REF=refs/heads/master GITHUB_RUN_ID=123456 GHCAP_RELEASE_MODE=preview
export MOCK_SHA="$COMMIT" MOCK_TAG="v0.0.31.0-pre.${COMMIT:0:12}" MOCK_LOG="$BASE/calls.log" MOCK_REMOTE_ASSETS="$BASE/remote-assets" MOCK_DRAFT_FLAG="$BASE/draft-created" MOCK_TAG_FLAG="$BASE/tag-created" MOCK_PUBLISHED_FLAG="$BASE/published" GH_TOKEN=fixture
printf '%s\n' "$COMMIT" > "$BASE/mac/SOURCE_COMMIT.txt"
printf '%s\n' "$COMMIT" > "$BASE/win/SOURCE_COMMIT.txt"
printf '%s\n' "$COMMIT" > "$BASE/linux/ghcap-v0.0.31.0-linux-x86_64/documentation/SOURCE_COMMIT.txt"
printf 'test executable\n' > "$BASE/win/ghcap.exe"
DIST="$ROOT/release-assets"
export MOCK_DIST="$DIST"
cp "$BASE/win/ghcap.exe" "$DIST/ghcap-v0.0.31.0-windows-x64.exe"
(cd "$BASE/win" && zip -q "$DIST/ghcap-v0.0.31.0-windows-x64.zip" SOURCE_COMMIT.txt ghcap.exe)
for arch in macos-intel macos-apple-silicon; do tar -C "$BASE/mac" -czf "$DIST/ghcap-v0.0.31.0-$arch.tar.gz" SOURCE_COMMIT.txt; done
tar -C "$BASE/linux" -czf "$DIST/ghcap-v0.0.31.0-linux-x86_64.tar.gz" ghcap-v0.0.31.0-linux-x86_64
for name in ghcap_0.0.31.0-1_amd64.deb ghcap-0.0.31.0-1.x86_64.rpm ghcap-0.0.31.0-1-x86_64.pkg.tar.zst ghcap-v0.0.31.0-linux-x86_64.AppImage; do printf 'fixture\n' > "$DIST/$name"; done
for p in "$DIST"/*; do (cd "$DIST" && sha256sum "${p##*/}" > "${p##*/}.sha256"); done
cat > "$BASE/bin/gh" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$MOCK_LOG"
args="$*"
case "$args" in
  *"git/matching-refs/tags"*) printf '[]\n' ;;
  *"--method POST"*"git/refs"*) touch "$MOCK_TAG_FLAG"; printf '{}\n' ;;
  *"commits/"*) printf '%s\n' "$MOCK_SHA" ;;
  *"releases?per_page=100"*) printf '[[]]\n' ;;
  *"--method POST"*"/releases --input"*)
    touch "$MOCK_DRAFT_FLAG"
    jq -n --arg tag "$MOCK_TAG" '{id:321,tag_name:$tag,draft:true,prerelease:true,assets:[]}'
    ;;
  *"--method PATCH"*"releases/321 --input"*)
    touch "$MOCK_PUBLISHED_FLAG"
    jq -n --arg tag "$MOCK_TAG" '{id:321,tag_name:$tag,draft:false,prerelease:true,assets:[]}'
    ;;
  *"releases/321"*)
    assets='[]'
    for p in "$MOCK_REMOTE_ASSETS"/*; do
      [[ -f "$p" ]] || continue
      name=${p##*/}
      digest="sha256:$(sha256sum "$p" | cut -d ' ' -f1)"
      assets=$(jq -c --arg name "$name" --arg digest "$digest" '. + [{name:$name,digest:$digest}]' <<< "$assets")
    done
    jq -n --arg tag "$MOCK_TAG" --argjson assets "$assets" '{id:321,tag_name:$tag,draft:true,prerelease:true,assets:$assets}'
    ;;
  *) echo "Unexpected mock gh call: $args" >&2; exit 96 ;;
esac
MOCK
cat > "$BASE/bin/curl" <<'MOCK_CURL'
#!/usr/bin/env bash
set -euo pipefail
args="$*"
name=${args##*?name=}
[[ "$name" =~ ^[A-Za-z0-9._-]+$ ]] || { echo "Invalid asset name: $name" >&2; exit 90; }
count=$(find "$MOCK_REMOTE_ASSETS" -maxdepth 1 -type f | wc -l)
if [[ "${MOCK_FAIL_AFTER:-0}" -gt 0 && "$count" -ge "$MOCK_FAIL_AFTER" ]]; then
  echo 'Simulated interrupted upload' >&2; exit 22
fi
cp -- "$MOCK_DIST/$name" "$MOCK_REMOTE_ASSETS/$name"
jq -n --arg name "$name" '{id:100,name:$name}'
MOCK_CURL
chmod 755 "$BASE/bin/gh" "$BASE/bin/curl"
export PATH="$BASE/bin:$PATH"
# EN: Interrupt halfway, then resume using existing draft ID and previously uploaded assets.
# JA: アップロード途中で失敗させ、数値IDを指定して既存Assetsを再利用できるか確認します。
export MOCK_FAIL_AFTER=7
if bash "$ROOT/packaging/publish-release.sh" > "$BASE/interrupted.txt" 2>&1; then
  echo 'Expected interrupted upload to fail' >&2; exit 1
fi
[[ $(find "$MOCK_REMOTE_ASSETS" -maxdepth 1 -type f | wc -l) == 7 ]] || { cat "$BASE/interrupted.txt" >&2; exit 1; }
rm -f "$DIST/ghcap-v0.0.31.0-source.tar.gz" "$DIST/ghcap-v0.0.31.0-source.tar.gz.sha256" "$DIST/SHA256SUMS"
export MOCK_FAIL_AFTER=0 GHCAP_RELEASE_ID=321
bash "$ROOT/packaging/publish-release.sh" > "$BASE/output.txt" 2>&1 || { cat "$BASE/output.txt" >&2; exit 1; }
[[ $(grep -c 'Reusing verified asset' "$BASE/output.txt") == 7 ]] || { echo 'Recovery did not reuse 7 verified assets' >&2; exit 1; }
[[ $(grep -c 'api --method POST repos/solintet/ghcap/releases --input' "$MOCK_LOG") == 1 ]] || { echo 'Duplicate Draft created' >&2; exit 1; }
[[ -e "$MOCK_TAG_FLAG" && -e "$MOCK_DRAFT_FLAG" && -e "$MOCK_PUBLISHED_FLAG" ]] || { echo 'Preview tag/draft/publish sequence not completed' >&2; exit 1; }
count=$(find "$MOCK_REMOTE_ASSETS" -maxdepth 1 -type f | wc -l)
[[ "$count" -eq 21 ]] || { echo "Expected 21 release assets, found $count" >&2; exit 1; }
[[ $(grep -c '^api repos/solintet/ghcap/releases/321' "$MOCK_LOG") -ge 21 ]] || { echo 'Release ID reads missing' >&2; exit 1; }
printf 'PASS / 成功: interrupted upload recovered by Release ID, 21 verified assets and publication\n'
