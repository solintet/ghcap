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
export MOCK_SHA="$COMMIT" MOCK_TAG="v0.0.31.0-pre.${COMMIT:0:12}" MOCK_LOG="$BASE/calls.log" MOCK_REMOTE_ASSETS="$BASE/remote-assets" MOCK_DRAFT_FLAG="$BASE/draft-created" MOCK_TAG_FLAG="$BASE/tag-created" MOCK_PUBLISHED_FLAG="$BASE/published"
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
# EN: Simulate GitHub's absence of a preview tag and draft existence after creation.
# JA: 未作成タグの照会後にタグとDraftが作成される状態を模擬します。
set -euo pipefail
printf '%s\n' "$*" >> "$MOCK_LOG"
case "$1" in
  api)
    args="$*"
    if [[ "$args" == *"git/matching-refs/tags/"* ]]; then
      echo 'ERROR: exact matching-refs URL must not be used for a missing preview tag' >&2
      exit 90
    elif [[ "$args" == *"git/matching-refs/tags"* ]]; then
      echo '[]'
    elif [[ "$args" == *"--method POST"* && "$args" == *"git/refs"* ]]; then
      touch "$MOCK_TAG_FLAG"
      echo '{}'
    elif [[ "$args" == *"commits/"* ]]; then
      printf '%s\n' "$MOCK_SHA"
    elif [[ "$args" == *"releases?per_page=100"* ]]; then
      if [[ -e "$MOCK_DRAFT_FLAG" ]]; then
        jq -n --arg tag "$MOCK_TAG" '[[{id:321,tag_name:$tag,draft:true,prerelease:true,assets:[]}]]'
      else
        printf '[[]]\n'
      fi
    elif [[ "$args" == *"releases/321"* ]]; then
      assets='[]'
      for p in "$MOCK_REMOTE_ASSETS"/*; do
        [[ -f "$p" ]] || continue
        name=${p##*/}
        digest="sha256:$(sha256sum "$p" | cut -d ' ' -f1)"
        assets=$(jq -c --arg name "$name" --arg digest "$digest" '. + [{name:$name,digest:$digest}]' <<< "$assets")
      done
      jq -n --arg tag "$MOCK_TAG" --argjson assets "$assets" '{id:321,tag_name:$tag,draft:true,prerelease:true,assets:$assets}'
    elif [[ "$args" == *"releases/tags/"* ]]; then
      echo 'ERROR: draft must be fetched from authenticated releases list' >&2
      exit 91
    else
      echo "ERROR: unexpected mock API: $args" >&2
      exit 92
    fi
    ;;
  release)
    action=$2
    case "$action" in
      create) [[ -e "$MOCK_TAG_FLAG" ]] || exit 93; touch "$MOCK_DRAFT_FLAG" ;;
      upload) cp -- "$4" "$MOCK_REMOTE_ASSETS/${4##*/}" ;;
      edit) touch "$MOCK_PUBLISHED_FLAG" ;;
      view) printf 'https://github.com/%s/releases/tag/%s\n' "$GITHUB_REPOSITORY" "$MOCK_TAG" ;;
      *) echo "ERROR: unexpected release action: $action" >&2; exit 94 ;;
    esac
    ;;
  *) echo "ERROR: unexpected command: $*" >&2; exit 95 ;;
esac
MOCK
chmod 755 "$BASE/bin/gh"
export PATH="$BASE/bin:$PATH"
bash "$ROOT/packaging/publish-release.sh" > "$BASE/output.txt" 2>&1 || { cat "$BASE/output.txt" >&2; exit 1; }
[[ -e "$MOCK_TAG_FLAG" && -e "$MOCK_DRAFT_FLAG" && -e "$MOCK_PUBLISHED_FLAG" ]] || { echo 'Preview tag/draft/publish sequence not completed' >&2; exit 1; }
count=$(find "$MOCK_REMOTE_ASSETS" -maxdepth 1 -type f | wc -l)
[[ "$count" -eq 21 ]] || { echo "Expected 21 release assets, found $count" >&2; exit 1; }
[[ $(grep -c '^release upload ' "$MOCK_LOG") -eq 21 ]] || { echo 'Unexpected release uploads' >&2; exit 1; }
printf 'PASS / 成功: new preview tag, draft, 21 verified assets and publication\n'
